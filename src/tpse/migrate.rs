use std::mem::take;
use std::pin::Pin;

use serde_json::{Value, json};

/// An interface to access a raw json storage backend for interacting with old versions of TPSEs
/// that need migration before they can be successfully parsed.
/// The simplest possible DynamicTPSE is implemented on [`serde_json::Value`]
#[allow(async_fn_in_trait)]
pub trait DynamicTPSE {
  type Error: std::error::Error;
  async fn get(&self, key: &str) -> Result<Option<Value>, Self::Error>;
  async fn set(&mut self, key: &str, value: Option<Value>) -> Result<(), Self::Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum ValueDynamicTPSEError {
  #[error("attempted to operate on a non-object json value")]
  JSONRootNotObject
}
impl DynamicTPSE for serde_json::Value {
  type Error = ValueDynamicTPSEError;
  
  async fn get(&self, key: &str) -> Result<Option<Value>, Self::Error> {
    let object = self.as_object().ok_or(ValueDynamicTPSEError::JSONRootNotObject)?;
    Ok(object.get(key).cloned())
  }

  async fn set(&mut self, key: &str, value: Option<Value>) -> Result<(), Self::Error> {
    let object = self.as_object_mut().ok_or(ValueDynamicTPSEError::JSONRootNotObject)?;
    match value {
      Some(value) => object.insert(key.to_string(), value),
      None => object.remove(key)
    };
    Ok(())
  }
}

#[derive(Debug, thiserror::Error)]
pub enum MigrationErrorKind<T: DynamicTPSE> {
  #[error("failed to {access:?} field {field}: {error}")]
  FieldAccessError { access: FieldAccess, error: T::Error, field: &'static str },
  #[error("attempted bad {cast} cast of tpse value {field}")]
  CastError { field: &'static str, cast: &'static str },
  #[error("failed to deserialize field {field}: {error}")]
  FieldDeserializeError { field: &'static str, error: serde_json::Error },
  #[error("failed to serialize new value: {0}")]
  SerializeError(serde_json::Error),
  #[error("failed to parse field or access subfield {field}")]
  FieldParseFailure { field: &'static str }
}
#[derive(Debug)]
pub enum FieldAccess { Read, Write }

struct Migration<T: DynamicTPSE> {
  version: Version,
  migrate: for<'a> fn(&'a mut T) -> Pin<Box<dyn Future<Output = Result<(), MigrationErrorKind<T>>> + 'a>>
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct Version {
  pub major: u32,
  pub minor: u32,
  pub patch: u32
}
impl std::fmt::Display for Version {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
  }
}
macro_rules! version {
  ($major:literal, $minor:literal, $patch:literal) => {
    Version { major: $major, minor: $minor, patch: $patch }
  }
}
#[cfg(test)] #[test]
fn assert_version_ordering() {
  // just a sanity check to make sure ord+partialord do what we expect
  let a = version!(0,10,0);
  let b = version!(0,17,0);
  let c = version!(0,17,1);
  let d = version!(1,0,0);
  assert!(a < b);
  assert!(b < c);
  assert!(c < d);
  assert!(b > a);
  assert!(c > b);
  assert!(d > c);
  assert!(!(a < a));
  assert!(a == a);
}

/// helper macro to make working with typed json a little less verbose
macro_rules! access {
  // helpers for use on DynamicTPSEs
  ($tpse:expr, get, $key:literal, as $binding:ident) => {
    access!(@internal, $tpse, get, $key, deserialize=, cast=, as $binding);
  };
  ($tpse:expr, get, $key:literal, $cast:ident, as $binding:ident) => {
    access!(@internal, $tpse, get, $key, deserialize=, cast=$cast, as $binding);
  };
  ($tpse:expr, get, $key:literal, deserialize, $cast:ident, as $binding:ident) => {
    access!(@internal, $tpse, get, $key, deserialize=_yes, cast=$cast, as $binding);
  };
  (@internal, $tpse:expr, get, $key:literal, deserialize=$($deserialize:ident)?, cast=$($cast:ident)?, as $binding:ident) => {
    #[allow(unused_mut)]
    let mut $binding = $tpse.get($key).await.map_err(|error| {
      MigrationErrorKind::FieldAccessError { access: FieldAccess::Read, error, field: $key }
    })?;
    $(
      // we need _something_ to ensure this optional group is correctly repeated,
      // but it doesn't matter what it actually is.
      let _group_repeat_anchor = stringify!($deserialize);
      // some tpse values are stringified json, so we have to do a second deserialization step here
      let mut $binding: Option<Value> = match $binding {
        None => None,
        Some(value) => {
          let string = value.as_str().ok_or(MigrationErrorKind::CastError { field: $key, cast: "as_str (deserialize)" })?;
          serde_json::from_str(string).map_err(|error| MigrationErrorKind::FieldDeserializeError { field: "", error })?
        }
      };
    )?
    $(
      let $binding = match &mut $binding {
        Some(value) => Some(value.$cast().ok_or(MigrationErrorKind::CastError { field: $key, cast: stringify!(cast) })?),
        None => None
      };
    )?
  };
  ($tpse:expr, get, $key:ident, $cast:ident, |$local:ident| $scope:expr) => {{
    access!($tpse, get, $key, $cast, as $local);
    $scope
  }};
  ($tpse:expr, set, $key:literal, $value:expr) => {{
    $tpse.set($key, $value).await.map_err(|error| {
      MigrationErrorKind::FieldAccessError { access: FieldAccess::Write, error, field: $key }
    })?
  }};
  ($tpse:expr, set, $key:literal, serialize, $value:expr) => {{
    let serialized = match $value {
      Some(value) => Some(serde_json::to_string(&value).map_err(|error| MigrationErrorKind::SerializeError(error))?.into()),
      None => None
    };
    access!($tpse, set, $key, serialized);
  }};
  
  // standalone helpers not for use on DynamicTPSEs
  (get, $object:expr, $key:literal, $path:literal$(, $cast:ident)?) => {{
    let value = $object.get_mut($key).ok_or(MigrationErrorKind::FieldParseFailure { field: $path })?;
    $( let value = access!(cast, value, $path, $cast); )?
    value
  }};
  (cast, $value:expr, $path:literal, $cast:ident) => {
    $value.$cast().ok_or(MigrationErrorKind::CastError {
      field: $path,
      cast: stringify!($cast)
    })?
  };
  (extend, $value:expr, $with:expr) => {{
    let Value::Object(extend) = $with else { unreachable!() };
    $value.extend(extend.into_iter());
  }};
}

const fn migrations<T>() -> [Migration<T>; 18] where T: DynamicTPSE {[
  /*
    v0.10.0 - Introduced migrations
    All this version adds is a version tag
  */
  Migration {
    version: version!(0,10,0),
    migrate: |_tpse| Box::pin(async {
      Ok(())
    })
  },
  
  /*
    v0.12.0 - Music graph update
    Adds a bunch of new keys to the music graph
  */
  Migration {
    version: version!(0,12,0),
    migrate: |tpse| Box::pin(async {
      access!(tpse, get, "musicGraph", deserialize, as_array_mut, as graph);
      let Some(graph) = graph else { return Ok(()) };
      for (i, node) in graph.iter_mut().enumerate() {
        let node = access!(cast, node, "graph[]", as_object_mut);
        access!(extend, node, json!({
          "x": i as f64 * 30.0,
          "y": 0,
          "effects": {
            "volume": 1,
            "speed": 1
          }
        }));
        
        for trigger in access!(get, node, "triggers", "graph[].triggers", as_array_mut) {
          let trigger = access!(cast, trigger, "graph[].triggers[]", as_object_mut);
          access!(extend, trigger, json!({
            "anchor": {
              "origin": { "x": 100, "y": 60 },
              "target": { "x": 100, "y": 0 }
            },
            "crossfade": false,
            "crossfadeDuration": 1,
            "locationMultiplier": 1
          }));
        }
      }
      access!(tpse, set, "musicGraph", serialize, Some(graph));
      Ok(())
    })
  },
  
  /*
    v0.13.0 - Music editor update
    Redesigned the music editor and added overrides
  */
  Migration {
    version: version!(0,13,0),
    migrate: |tpse| Box::pin(async {
      access!(tpse, get, "music", as_array_mut, as music);
      let Some(music) = music else { return Ok(()) };
      for song in music.iter_mut() {
        access!(cast, song, "music[]", as_object_mut).insert("override".to_string(), Value::Null);
      }
      access!(tpse, set, "music", Some(take(music).into()));
      Ok(())
    })
  },
  
  /*
    v0.14.0 - TPSE integration update
    Added the 'useContentPack' URL-based loader.
  */
  Migration {
    version: version!(0,14,0),
    migrate: |tpse| Box::pin(async {
      access!(tpse, set, "whitelistedLoaderDomains", Some(json!([
        "# One protocol and domain per",
        "# line. https recommended.",
        "https://tetrio.team2xh.net",
        "https://you.have.fail"
      ])));
      Ok(())
    })
  },
  
  /*
    v0.15.0 - Better:tm: skins update
    'skin' -> 'skinSvg'
  */
  Migration {
    version: version!(0,15,0),
    migrate: |tpse| Box::pin(async {
      access!(tpse, get, "skin", as skin);
      access!(tpse, set, "skin", None);
      access!(tpse, set, "skinSvg", skin);
      Ok(())
    })
  },
  
  /*
    v0.17.0 - Small music graph update + general bugfixes
    Added:
    - tetrioPlusEnabled
    - musicGraph[].audioStart
    - musicGraph[].audioEnd
  */
  Migration {
    version: version!(0,17,0),
    migrate: |tpse| Box::pin(async {
      access!(tpse, set, "tetrioPlusEnabled", Some(true.into()));
      
      access!(tpse, get, "musicGraph", deserialize, as_array_mut, as graph);
      let Some(graph) = graph else { return Ok(()) };
      for node in graph.iter_mut() {
        let node = access!(cast, node, "graph[]", as_object_mut);
        access!(extend, node, json!({
          "audioStart": 0,
          "audioEnd": 0
        }));
      }
      access!(tpse, set, "musicGraph", serialize, Some(graph));
      
      Ok(())
    })
  },
  
  /*
    v0.18.0 - (more) Small music graph update + general bugfixes
    added:
    - musicGraph[].triggers[].dispatchEvent
    - musicGraph[].background
  */
  Migration {
    version: version!(0,18,0),
    migrate: |tpse| Box::pin(async {
      access!(tpse, get, "musicGraph", deserialize, as_array_mut, as graph);
      let Some(graph) = graph else { return Ok(()) };
      for node in graph.iter_mut() {
        let node = access!(cast, node, "graph[]", as_object_mut);
        node.insert("background".to_string(), Value::Null);
        
        for trigger in access!(get, node, "triggers", "graph[].triggers", as_array_mut) {
          let trigger = access!(cast, trigger, "graph[].triggers[]", as_object_mut);
          trigger.insert("dispatchEvent".to_string(), "".into());
        }
      }
      access!(tpse, set, "musicGraph", serialize, Some(graph));
      
      Ok(())
    })
  },
  
  /*
    v0.18.2 - Partial update for new skin format
    Removed: skinSvg, skinPng, skinAnim, skinAnimMeta
    Added: skin, ghost
  */
  Migration {
    version: version!(0,18,2),
    migrate: |tpse| Box::pin(async {
      // TODO: this other todo copied from the original migrate.js
      // // TODO: Implement a real migration for this data
      // // (importers are es6, migrate.js unfortunately isn't.)
      access!(tpse, set, "skinSvg", None);
      access!(tpse, set, "skinPng", None);
      access!(tpse, set, "skinAnim", None);
      access!(tpse, set, "skinAnimMeta", None);
      
      Ok(())
    })
  },
  
  /*
    v0.20.0 - More music graph stuff
    added:
    - musicGraph[].backgroundLayer
  */
  Migration {
    version: version!(0,20,0),
    migrate: |tpse| Box::pin(async {
      access!(tpse, get, "musicGraph", deserialize, as_array_mut, as graph);
      let Some(graph) = graph else { return Ok(()) };
      for node in graph.iter_mut() {
        let node = access!(cast, node, "graph[]", as_object_mut);
        node.insert("backgroundLayer".to_string(), (0.0).into());
      }
      access!(tpse, set, "musicGraph", serialize, Some(graph));
      Ok(())
    })
  },
  
  /*
    v0.21.0 - Even more music graph stuff
    added:
    - musicGraph[].triggers[].expression
    - musicGraph[].triggers[].variable
  */
  Migration {
    version: version!(0,21,0),
    migrate: |tpse| Box::pin(async {
      access!(tpse, get, "musicGraph", deserialize, as_array_mut, as graph);
      let Some(graph) = graph else { return Ok(()) };
      
      for node in graph.iter_mut() {
        let node = access!(cast, node, "graph[]", as_object_mut);
        
        for trigger in access!(get, node, "triggers", "graph[].triggers", as_array_mut) {
          let trigger = access!(cast, trigger, "graph[].triggers[]", as_object_mut);
          trigger.insert("expression".to_string(), "".into());
          trigger.insert("variable".to_string(), "".into());
        }
      }
      access!(tpse, set, "musicGraph", serialize, Some(graph));
      
      Ok(())
    })
  },
  
  /*
    v0.20.1 - Music graph variables betterer

    + musicGraph[].triggers[].predicateExpression
    = musicGraph[].triggers[].value -> timePassedDuration or predicate
    = musicGraph[].triggers[].valueOperator -> predicate
    = musicGraph[].triggers[].expression -> setExpression, dispatchExpression
    = musicGraph[].triggers[].variable -> setVariable
  */
  Migration {
    version: version!(0,20,1),
    migrate: |tpse| Box::pin(async {
      
      access!(tpse, get, "musicGraph", deserialize, as_array_mut, as graph);
      let Some(graph) = graph else { return Ok(()) };
      
      let event_value_extended_modes = [
        "fx-countdown",
        "fx-offense-player",
        "fx-offense-enemy",
        "fx-defense-player",
        "fx-defense-enemy",
        "fx-combo-player",
        "fx-combo-enemy",
        "fx-line-clear-player",
        "fx-line-clear-enemy",
        "board-height-player",
        "board-height-enemy",
      ];
      
      for node in graph.iter_mut() {
        let node = access!(cast, node, "graph[]", as_object_mut);
        node.insert("background".to_string(), Value::Null);
        
        for trigger in access!(get, node, "triggers", "graph[].triggers", as_array_mut) {
          let trigger = access!(cast, trigger, "graph[].triggers[]", as_object_mut);
          
          let event = access!(get, trigger, "event", "graph[].triggers[].event", as_str).to_string();
          let value = access!(get, trigger, "value", "graph[].triggers[].value", as_number).as_f64().unwrap_or(0.0);
          let value_operator = access!(get, trigger, "valueOperator", "graph[].triggers[].valueOperator", as_str).to_string();
          let mode = access!(get, trigger, "mode", "graph[].triggers[].mode", as_str).to_string();
          let expression = access!(get, trigger, "expression", "graph[].triggers[].expression", as_str).to_string();
          let variable = access!(get, trigger, "variable", "graph[].triggers[].variable", as_str).to_string();
          
          access!(extend, trigger, json!({
            "timePassedDuration":
              (event == "repeating-time-passed" || event == "time-passed")
              .then(|| value).unwrap_or(0.0),
            "predicateExpression":
              (event_value_extended_modes.contains(&&event[..]) && value_operator != "any")
                .then(|| format!("$ {value_operator} {value}")).unwrap_or_default(),
            "dispatchExpression": (mode == "dispatch").then(|| expression.clone()).unwrap_or_default(),
            "setExpression": (mode == "set").then(|| expression.clone()).unwrap_or_default(),
            "setVariable": variable,
          }));
          trigger.remove("value");
          trigger.remove("valueOperator");
          trigger.remove("expression");
          trigger.remove("variable");
        }
      }
      access!(tpse, set, "musicGraph", serialize, Some(graph));
      
      Ok(())
    })
  },
  
  /*
    v0.21.3 - Slightly Better Backgrounds
    added:
    - backgrounds[].type
  */
  Migration {
    version: version!(0,21,3),
    migrate: |tpse| Box::pin(async {
      access!(tpse, get, "backgrounds", as_array_mut, as backgrounds);
      let Some(backgrounds) = backgrounds else { return Ok(()) };
      
      for background in backgrounds.iter_mut() {
        access!(cast, background, "backgrounds[]", as_object_mut)
          .insert("type".to_string(), "image".into());
      }
      
      access!(tpse, set, "backgrounds", Some(take(backgrounds).into()));
      Ok(())
    })
  },
  
  /*
    v0.23.4 - More music graph stuff
    + musicGraph[].singleInstance
  */
  Migration {
    version: version!(0,23,4),
    migrate: |tpse| Box::pin(async {
      access!(tpse, get, "musicGraph", deserialize, as_array_mut, as graph);
      let Some(graph) = graph else { return Ok(()) };
      
      for node in graph.iter_mut() {
        let node = access!(cast, node, "graph[]", as_object_mut);
        node.insert("singleInstance".into(), false.into());
      }
      access!(tpse, set, "musicGraph", serialize, Some(graph));
      
      Ok(())
    })
  },
  
  /*
    v0.23.8 - Winter compat patch
    + winterCompatEnabled
  */
  Migration {
    version: version!(0,23,8),
    migrate: |_tpse| Box::pin(async {
      // THIS MIGRATION RETROACTIVELY REMOVED
      // (during the rewrite of migrate.js)
      // (why did it even exist?)
      Ok(())
    })
  },
  
  /*
    v0.25.3 - Music graph foregrounds
    + musicGraph[].backgroundArea
  */
  Migration {
    version: version!(0,25,3),
    migrate: |tpse| Box::pin(async {
      access!(tpse, get, "musicGraph", deserialize, as_array_mut, as graph);
      let Some(graph) = graph else { return Ok(()) };
      
      for node in graph.iter_mut() {
        let node = access!(cast, node, "graph[]", as_object_mut);
        node.insert("backgroundArea".into(), "background".into());
      }
      access!(tpse, set, "musicGraph", serialize, Some(graph));
      
      Ok(())
    })
  },
  
  /*
    v0.27.3 - TETR.IO beta v1.0.0 adds 'hidden' field to music
    + music[].metadata.hidden
  */
  Migration {
    version: version!(0,27,3),
    migrate: |tpse| Box::pin(async {
      access!(tpse, get, "music", as_array_mut, as music);
      let Some(music) = music else { return Ok(()) };
      for song in music.iter_mut() {
        let song = access!(cast, song, "music[]", as_object_mut);
        let metadata = access!(get, song, "metadata", "music[].metadata", as_object_mut);
        metadata.insert("hidden".to_string(), false.into());
      }
      access!(tpse, set, "music", Some(take(music).into()));
      Ok(())
    })
  },
  
  /*
    v0.27.10 - TETR.IO beta β1.7.4  adds 'normalizeDb' field to music
    + music[].metadata.normalizeDb
  */
  Migration {
    version: version!(0,27,10),
    migrate: |tpse| Box::pin(async {
      access!(tpse, get, "music", as_array_mut, as music);
      let Some(music) = music else { return Ok(()) };
      for song in music.iter_mut() {
        let song = access!(cast, song, "music[]", as_object_mut);
        let metadata = access!(get, song, "metadata", "music[].metadata", as_object_mut);
        metadata.insert("normalizeDb".to_string(), (0.0).into());
      }
      access!(tpse, set, "music", Some(take(music).into()));
      Ok(())
    })
  },
  
  /*
    v0.28.0 - flattens double serialization of the music graph and touch controls.
    It's weird and more cumbersome to deal with in rust than it is in javascript.
    + JSON.deserialize(musicGraph)
    + JSON.deserialize(touchControlConfig)
  */
  Migration {
    version: version!(0,28,0),
    migrate: |tpse| Box::pin(async {
      access!(tpse, get, "musicGraph", deserialize, as_array_mut, as graph);
      if let Some(graph) = graph {
        access!(tpse, set, "musicGraph", /* no `serialize,` ! */ Some(take(graph).into()));
      }
      
      
      access!(tpse, get, "touchControlConfig", deserialize, as_array_mut, as graph);
      if let Some(graph) = graph {
        access!(tpse, set, "touchControlConfig", /* no `serialize,` ! */ Some(take(graph).into()));
      }
      Ok(())
    })
  }
]}

#[cfg(test)] #[tokio::test]
async fn migration_test() {
  use crate::tpse::TPSE;

  let mut value = json!({
    "musicGraph": serde_json::to_string(&json!([])).unwrap()
  });
  
  migrate(&mut value).await.unwrap();
  
  let tpse: TPSE = serde_json::from_value(value).unwrap();
  assert_eq!(
    tpse.version.unwrap(),
    migrations::<Value>().last().unwrap().version.to_string()
  );
}

#[derive(Debug, thiserror::Error)]
pub enum MigrationError<T: DynamicTPSE> {
  #[error("error while preparing migration: {0}")]
  Setup(#[source] MigrationErrorKind<T>),
  #[error("error in migration v{0}: {1}")]
  Run(Version, #[source] MigrationErrorKind<T>)
}

async fn parse_version<T>(tpse: &mut T) -> Result<Version, MigrationErrorKind<T>> where T: DynamicTPSE {
  access!(tpse, get, "version", as_str, as version);
  match version {
    None => Ok(version!(0,0,0)),
    Some(version) => {
      let mut iter = version.split(".");
      let err = ||MigrationErrorKind::FieldParseFailure { field: "version" };
      let version = Version {
        major: iter.next().ok_or(err())?.parse().map_err(|_| err())?,
        minor: iter.next().ok_or(err())?.parse().map_err(|_| err())?,
        patch: iter.next().ok_or(err())?.parse().map_err(|_| err())?
      };
      if iter.next().is_some() {
        return Err(err());
      }
      Ok(version)
    }
  }
}
async fn set_version<T>(tpse: &mut T, version: Version) -> Result<(), MigrationErrorKind<T>> where T: DynamicTPSE {
  access!(tpse, set, "version", Some(version.to_string().into()));
  Ok(())
}


pub async fn migrate<T>(tpse: &mut T) -> Result<(), MigrationError<T>> where T: DynamicTPSE {
  let parsed_version = parse_version(tpse).await.map_err(|err| MigrationError::Setup(err))?;
  for migration in migrations() {
    if parsed_version < migration.version {
      (migration.migrate)(tpse).await.map_err(|err| MigrationError::Run(migration.version, err))?;
      set_version(tpse, migration.version).await.map_err(|err| MigrationError::Run(migration.version, err))?;
    }
  }
  Ok(())
}