use std::borrow::Cow;
use std::collections::HashMap;

use crate::tpse::music_graph::Node;
use crate::tpse::{AnimMeta, AnimatedBackground, Background, CustomSoundAtlas, File, MiscTPSEValue, Song, TouchControlConfig};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{DeserializeOwned, Error as _};
use serde::ser::Error as _;

/// An index into a single value of tetrio plus configuration data.
/// A TPSEKey describes the key (possibly parametrically), the data stored there, and how to merge the
/// data between two [TPSEProvider]s.
pub trait TPSEKey: Clone + Sized {
  type Data: Serialize + DeserializeOwned + Clone;
  fn key(&self) -> &str;
}

/// A TPSEProvider is an interface to an abstract store of tetrio plus data indexed by [TPSEKey]s.
/// The basic form of TPSEProvider is the [TPSE] struct, which stores everything in one big struct in memory.
/// More complex forms of TPSEProvider involve moving data key-by-key to and from an external source such
/// as across wasm boundaries with tetrio plus's `browser.storage.local`.
pub trait TPSEProvider<T: TPSEKey> {
  fn get(&self, key: &T) -> Option<Cow<'_, T::Data>>;
  fn set(&mut self, key: &T, value: Option<T::Data>);
}

macro_rules! merge_logic {
  ($key:expr, $base:expr, $source:expr) => {
    let value = $source.get(&$key).or_else(|| $base.get(&$key));
    if let Some(value) = value {
      $base.set(&$key, Some(value.into_owned()));
    }
  };
  ($key:expr, $base:expr, $source:expr, $custom_merge_logic:expr) => {
    $custom_merge_logic($base, $source)
  };
}

/// Initializes most of the [TPSE] struct and accompanying [TPSEKey]s for every field in it
macro_rules! tpse_keys {
  ([
    $(
      $(#[$($extra_annotations:tt)+])*
      ($name:ident, $key:expr, $data:ty $(, $custom_merge_logic:expr)?)
    ),*
  ], {
    extra_struct_keys={$($extra_struct_keys:tt)+},
    extra_merge_bounds={$($extra_bounds:tt)+}
  }) => {
    $(
      #[allow(unused, non_camel_case_types)]
      #[derive(Clone, Debug)]
      pub struct $name;
      impl TPSEKey for $name {
        type Data = $data;
        fn key(&self) -> &str {
          $key
        }
      }
      impl TPSEProvider<$name> for TPSE {
        fn get(&self, _key: &$name) -> Option<Cow<'_, $data>> {
          self.$name.as_ref().map(|x| Cow::Borrowed(x))
        }
        fn set(&mut self, _key: &$name, value: Option<$data>) {
          self.$name = value;
        }
      }
    )+
    
    #[allow(unused)]
    #[serde_with::skip_serializing_none]
    #[serde_with::serde_as]
    #[derive(Debug, Default, Serialize, Deserialize)]
    pub struct TPSE {
      $(
        $(#[$($extra_annotations)+])*
        pub $name: Option<$data>,
      )+
      $($extra_struct_keys)+
    }

    // non-plain keys are merged by the specialized behavior of the [backgrounds] and [music] keys
    pub fn merge<A, B>(base: &mut A, source: &B) where
      $(A: TPSEProvider<$name>,)+
      $(B: TPSEProvider<$name>,)+
      A: $($extra_bounds)+,
      B: $($extra_bounds)+,
    {
      $( merge_logic!($name, base, source$(, $custom_merge_logic)?); )+
    }
  }
}

tpse_keys!([
  (sfx_enabled, "sfxEnabled", bool),
  (music_enabled, "musicEnabled", bool),
  (music_graph_enabled, "musicGraphEnabled", bool),
  (disable_vanilla_music, "disableVanillaMusic", bool),
  (enable_missing_music_patch, "enableMissingMusicPatch", bool),
  (enable_osd, "enableOSD", bool),
  (bg_enabled, "bgEnabled", bool),
  (animated_bg_enabled, "animatedBgEnabled", bool),
  (enable_touch_controls, "enableTouchControls", bool),
  (enable_emote_tab, "enableEmoteTab", bool),
  (watermark_enabled, "watermarkEnabled", bool),
  (transparent_bg_enabled, "transparentBgEnabled", bool),
  (opaque_transparent_background, "opaqueTransparentBackground", bool),
  (open_devtools_on_start, "openDevtoolsOnStart", bool),
  (tetrio_plus_enabled, "tetrioPlusEnabled", bool),
  (hide_tetrio_plus_on_startup, "hideTetrioPlusOnStartup", bool),
  (allow_url_pack_loader, "allowURLPackLoader", bool),
  (whitelisted_loader_domains, "whitelistedLoaderDomains", Vec<String>),
  (enable_custom_css, "enableCustomCss", bool),
  (custom_css, "customCss", String),
  (enable_all_song_tweaker, "enableAllSongTweaker", bool),
  (show_legacy_options, "showLegacyOptions", bool),
  (bypass_bootstrapper, "bypassBootstrapper", bool),
  (enable_custom_maps, "enableCustomMaps", bool),
  (advanced_skin_loading, "advancedSkinLoading", bool),
  (force_nearest_scaling, "forceNearestScaling", bool),
  (window_title_status, "windowTitleStatus", bool),
  (music_graph_background, "musicGraphBackground", bool),
  (board, "board", File),
  (winter_compat_enabled, "winterCompatEnabled", bool),
  (queue, "queue", File),
  (grid, "grid", File),
  (particle_beam, "particle_beam", File),
  (particle_beams_beam, "particle_beams_beam", File),
  (particle_bigbox, "particle_bigbox", File),
  (particle_box, "particle_box", File),
  (particle_chip, "particle_chip", File),
  (particle_chirp, "particle_chirp", File),
  (particle_dust, "particle_dust", File),
  (particle_fbox, "particle_fbox", File),
  (particle_fire, "particle_fire", File),
  (particle_particle, "particle_particle", File),
  (particle_smoke, "particle_smoke", File),
  (particle_star, "particle_star", File),
  (particle_flake, "particle_flake", File),
  (rank_d, "rank_d", File),
  (rank_dplus, "rank_dplus", File),
  (rank_cminus, "rank_cminus", File),
  (rank_c, "rank_c", File),
  (rank_cplus, "rank_cplus", File),
  (rank_bminus, "rank_bminus", File),
  (rank_b, "rank_b", File),
  (rank_bplus, "rank_bplus", File),
  (rank_aminus, "rank_aminus", File),
  (rank_a, "rank_a", File),
  (rank_aplus, "rank_aplus", File),
  (rank_sminus, "rank_sminus", File),
  (rank_s, "rank_s", File),
  (rank_splus, "rank_splus", File),
  (rank_ss, "rank_ss", File),
  (rank_u, "rank_u", File),
  (rank_x, "rank_x", File),
  (rank_z, "rank_z", File),
  (skin, "skin", File),
  (ghost, "ghost", File),
  (skin_anim, "skinAnim", File),
  (ghost_anim, "ghostAnim", File),
  (skin_anim_meta, "skinAnimMeta", AnimMeta),
  (ghost_anim_meta, "ghostAnimMeta", AnimMeta),
  (custom_sound_atlas, "customSoundAtlas", CustomSoundAtlas),
  (custom_sounds, "customSounds", File),
  (backgrounds, "backgrounds", Vec<Background>, merge_backgrounds),
  (animated_background, "animatedBackground", AnimatedBackground),
  (music, "music", Vec<Song>, merge_music),
  (music_graph, "musicGraph", Vec<Node>, merge_music_graphs),
  #[serde(deserialize_with = "deserialize_as_string", serialize_with = "serialize_as_string")]
  (touch_control_config, "touchControlConfig", TouchControlConfig)
], {
  extra_struct_keys={
    /// Other TPSE keys
    /// These should mainly be files for background and music IDs
    #[serde(flatten)]
    pub other: HashMap<String, MiscTPSEValue>
  },
  extra_merge_bounds={TPSEProvider<IDFileEntry>}
});

fn serialize_as_string<T, S>(value: &T, ser: S) -> Result<S::Ok, S::Error> where T: Serialize, S: Serializer {
  match serde_json::to_string(value) {
    Ok(string) => Ok(ser.serialize_str(&string)?),
    Err(err) => Err(S::Error::custom(err))
  }
}

fn deserialize_as_string<'a, T, D>(de: D) -> Result<T, D::Error> where T: DeserializeOwned, D: Deserializer<'a> {
  serde_json::from_str(&String::deserialize(de)?).map_err(D::Error::custom)
}

fn merge_music<A, B>(base: &mut A, source: &B) where
  A: TPSEProvider<music> + TPSEProvider<IDFileEntry>,
  B: TPSEProvider<music> + TPSEProvider<IDFileEntry>
{
  let merged = source.get(&music).iter()
    .chain(base.get(&music).iter())
    .flat_map(|x| x.as_ref())
    .cloned()
    .collect::<Vec<_>>();
  if merged.is_empty() { return }
  base.set(&music, Some(merged));
  for extra in source.get(&music).iter().flat_map(|x| x.as_ref()) {
    let key = IDFileEntry::new_song(&extra.id);
    base.set(&key, source.get(&key).map(|x| x.into_owned()));
  }
}
fn merge_backgrounds<A, B>(base: &mut A, source: &B) where
  A: TPSEProvider<backgrounds> + TPSEProvider<IDFileEntry>,
  B: TPSEProvider<backgrounds> + TPSEProvider<IDFileEntry>
{
  let merged = source.get(&backgrounds).iter()
    .chain(base.get(&backgrounds).iter())
    .flat_map(|x| x.as_ref())
    .cloned()
    .collect::<Vec<_>>();
  if merged.is_empty() { return }
  base.set(&backgrounds, Some(merged));
  for extra in source.get(&backgrounds).iter().flat_map(|x| x.as_ref()) {
    let key = IDFileEntry::new_background(&extra.id);
    base.set(&key, source.get(&key).map(|x| x.into_owned()));
  }
}
fn merge_music_graphs(base: &mut impl TPSEProvider<music_graph>, source: &impl TPSEProvider<music_graph>) {
  let Some(mut other_graph) = source.get(&music_graph).map(|x| x.into_owned()) else { return };
  let new_graph = match base.get(&music_graph).map(|x| x.into_owned()) {
    Some(mut self_graph) => {
      let max_id = self_graph.iter().map(|v| v.id).max().unwrap_or(0);
      let mut remapped_ids = HashMap::new();
      // Assign new IDs
      for (i, node) in other_graph.iter_mut().enumerate() {
        let new_id = max_id + i as u64 + 1;
        remapped_ids.insert(node.id, new_id);
        node.id = new_id;
      }
      // Update all ID references
      for node in &mut other_graph {
        for trigger in &mut node.triggers {
          trigger.target = remapped_ids.get(&trigger.target).copied().unwrap_or(0);
        }
      }
      // Merge the graphs
      self_graph.extend(other_graph);
      self_graph
    },
    None => other_graph
  };
  base.set(&music_graph, Some(new_graph));
}


/// A parametric key for accessing `song-{}` and `background-{}` TPSE keys
#[derive(Clone, Debug)]
pub struct IDFileEntry(String);
impl IDFileEntry {
  pub fn new_song(id: &str) -> Self {
    Self(format!("song-{id}"))
  }
  pub fn new_background(id: &str) -> Self {
    Self(format!("background-{id}"))
  }
}
impl TPSEKey for IDFileEntry {
  type Data = File;
  fn key(&self) -> &str {
    &self.0
  }
}
impl TPSEProvider<IDFileEntry> for TPSE {
  fn get(&self, key: &IDFileEntry) -> Option<Cow<'_, File>> {
    let entry = self.other.get(&key.0)?;
    let file = match entry {
      MiscTPSEValue::Other(_) => return None,
      MiscTPSEValue::File(file) => file,
    };
    Some(Cow::Borrowed(file))
  }
  fn set(&mut self, key: &IDFileEntry, value: Option<File>) {
    match value {
      Some(value) => self.other.insert(key.0.clone(), MiscTPSEValue::File(value)),
      None => self.other.remove(&key.0)
    };
  }
}

#[test]
fn null_merge_test() {
  struct NullTPSEProvider;
  impl<T: TPSEKey> TPSEProvider<T> for NullTPSEProvider {
    fn get(&self, _key: &T) -> Option<Cow<'_, T::Data>> { None }
    // into the bitbucket it goes
    fn set(&mut self, _key: &T, _value: Option<T::Data>) { unreachable!(); }
  }
  
  merge(&mut NullTPSEProvider, &NullTPSEProvider);
}