use std::collections::HashMap;
use std::path::PathBuf;

use itertools::Itertools;

use crate::accel::traits::TPSEAccelerator;
use crate::import::packjson::{ImportSet, ImportSetOption, PackJSON};
use crate::import::{ImportContext, ImportContextEntry, ImportError, ImportErrorType};
use crate::import::inter_stage_data::{DecisionTree, DecisionTreeEntry, ProcessedQueuedFile, SpecificImportTypeWithPackJsonAndUnknown};
use crate::log::LogLevel;

pub fn partition_import_groups<'a, T: TPSEAccelerator>
  (results: &'a [ProcessedQueuedFile], ctx: &mut ImportContext<'_, T>)
   -> Result<Vec<DecisionTree<'a>>, ImportError<T>>
{
  let mut loose_files = vec![];
  let mut pack_json_roots = vec![];
  struct PackJsonRootEntry {
    pack_file: PackJSON,
    pack_file_index: usize,
    pack_file_dir: PathBuf,
    children: Vec<PackJsonRootEntryChild>
  }
  struct PackJsonRootEntryChild {
    file_index: usize,
    matched_filters: Vec<MatchedFilter>
  }
  struct MatchedFilter {
    group_name: String,
    pattern_index: usize
  }

  // identify and load pack.json files
  for (i, file) in results.iter().enumerate() {
    if file.specific_kind == SpecificImportTypeWithPackJsonAndUnknown::PackJson {
      let parent = file.path.parent().expect("pack.json files are always named pack.json and so must have a parent");
      let pack_json = serde_json::from_slice(&file.binary)
        .map_err(|err| ctx.wrap_error(ImportErrorType::InvalidPackJson(file.path.clone(), err)))?;
      pack_json_roots.push(PackJsonRootEntry {
        pack_file: pack_json,
        pack_file_index: i,
        pack_file_dir: parent.to_path_buf(),
        children: vec![]
      });
    }
  }
  
  // assign each file to its nearest pack.json
  for (file_index, file) in results.iter().enumerate() {
    let effective_path = match file.specific_kind {
      // pack.json files make all files under their influence (i.e. in their directory) look like a single file
      // named as that directory from the perspective of pack.json files higher in the hierarchy.
      SpecificImportTypeWithPackJsonAndUnknown::PackJson => file.path.parent().expect("pack.json files are always named pack.json and so must have a parent"),
      _ => &file.path
    };
    
    let mut longest_path = 0;
    let mut longest_path_index = None;
    for (root_index, root) in pack_json_roots.iter().enumerate() {
      if effective_path == root.pack_file_dir { continue } // pack files don't influence themselves
      if !effective_path.starts_with(&root.pack_file_dir) { continue }
      if root.pack_file_dir.as_os_str().len() < longest_path { continue }
      longest_path = root.pack_file_dir.as_os_str().len();
      longest_path_index = Some(root_index);
    }
    
    if let Some(longest_path_index) = longest_path_index {
      pack_json_roots[longest_path_index].children.push(PackJsonRootEntryChild {
        file_index,
        matched_filters: vec![]
      });
    } else {
      // pack.json files are never considered 'loose' as they're transformed into the `DecisionTree`s
      // that actually determine which files to load.
      if file.specific_kind != SpecificImportTypeWithPackJsonAndUnknown::PackJson {
        loose_files.push(file);
      }
    }
  }
  
  // if there's no pack.json files, return a default tree that imports all files
  if pack_json_roots.is_empty() {
    return Ok(vec![DecisionTree {
      description: "content pack".to_string(),
      options: vec![DecisionTreeEntry {
        description: "all content pack data".to_string(),
        files: loose_files,
        subtrees: vec![]
      }]
    }]);
  }
  
  if !loose_files.is_empty() {
    ctx.log(LogLevel::Warn, &format_args!(
      "{} loose files which will be imported unconditionally. Consider adding them to a pack.json. Files: {:?}",
      loose_files.len(), loose_files.iter().map(|f| &f.path).format(", ")
    ));
  }
  
  struct FlatDecisionTree {
    description: String,
    options: Vec<FlatDecisionTreeEntry>,
  }
  struct FlatDecisionTreeEntry {
    description: String,
    files: Vec<usize>,
    subtrees: Vec<usize>
  }
  
  
  let mut trees = vec![];
  // a map of json.pack file index (in results) to index in tree list
  let mut jsonpack_file_to_tree_mapping = HashMap::new();
  for pack_json in pack_json_roots {
    jsonpack_file_to_tree_mapping.insert(pack_json.pack_file_index, trees.len());
    let tree = FlatDecisionTree {
      description: pack_json.pack_file.description.clone(),
      options: vec![]
    };
    trees.push((pack_json, tree));
  }
  // Tracks which trees are "rootlike", i.e. those which are not a subtree of another tree
  let mut trees_rootlike = vec![true; trees.len()];
  
  // Organize files into collections based on which groups they match and which import sets use those groups
  for (pack_json, tree) in &mut trees {
    let ctx = ctx.enter_context(ImportContextEntry::PackJson {
      pack_json_file: results[pack_json.pack_file_index].path.clone()
    });
    let mut groups = HashMap::with_capacity(pack_json.pack_file.import_groups.len());
    for (group_id, group_patterns) in &pack_json.pack_file.import_groups {
      for (pattern_index, pattern) in group_patterns.iter().enumerate() {
        for child in &mut pack_json.children {
          let relative_to_pack_json = results[child.file_index].path.strip_prefix(&pack_json.pack_file_dir)
            .expect("child path must be prefixed with pack_file_dir for it to be a child");
            
          if pattern.get_compiled_pattern().is_match(relative_to_pack_json) {
            child.matched_filters.push(MatchedFilter {
              group_name: group_id.clone(),
              pattern_index
            });
          }
        }
      }
      groups.insert(group_id, vec![]);
    }
    
    for child in &pack_json.children {
      if child.matched_filters.is_empty() {
        ctx.log(LogLevel::Warn, &format_args!(
          "File {:?} matched no filters in its nearest pack.json and will be considered loose",
          results[child.file_index].path
        ));
        loose_files.push(&results[child.file_index]);
      }
      if child.matched_filters.len() > 1 {
        ctx.log(LogLevel::Warn, &format_args!(
          "File {:?} matched multiple groups in its nearest pack.json. Consider adding a new group that uniquely matches this file and then add that group to each applicable import set. Matched groups: {}.",
          results[child.file_index].path,
          child.matched_filters.iter().format_with(", ", |filter, f| {
            f(&format_args!("'{}' (pattern #{})", filter.group_name, filter.pattern_index))
          })
        ));
      }
      for filter in &child.matched_filters {
        groups.get_mut(&filter.group_name).unwrap().push(child.file_index);
      }
    }
    
    for (group_id, group) in &groups {
      if group.is_empty() {
        ctx.log(LogLevel::Warn, &format_args!(
          "group {group_id} from {:?}/pack.json matches no files",
          pack_json.pack_file_dir
        ));
      }
    }
    let import_sets = match &pack_json.pack_file.import_sets {
      Some(sets) => &sets[..],
      None => &[ImportSet {
        title: "all content".to_string(),
        required: true,
        options: vec![ImportSetOption {
          description: "all files".to_string(),
          enables_groups: groups.keys().map(|x| x.to_string()).collect()
        }]
      }]
    };
    for set in import_sets {
      for option in &set.options {
        let mut entry = FlatDecisionTreeEntry {
          description: option.description.clone(),
          files: vec![],
          subtrees: vec![]
        };
        for group in &option.enables_groups {
          for file in groups.get(&group).unwrap() {
            if results[*file].specific_kind == SpecificImportTypeWithPackJsonAndUnknown::PackJson {
              let subtree = *jsonpack_file_to_tree_mapping.get(file).unwrap();
              entry.subtrees.push(subtree);
              trees_rootlike[subtree] = false;
            } else {
              entry.files.push(*file);
            }
          }
        }
        tree.options.push(entry);
      }
    }
  }
  
  // Erect those collections into one or more unified trees based on pack.json nesting
  let mut rooted_trees = vec![];
  let mut trees = trees.into_iter().map(|tree| Some(tree)).collect::<Vec<_>>();
  for (i, is_rootlike) in trees_rootlike.into_iter().enumerate() {
    if !is_rootlike { continue }
    rooted_trees.push(grow_tree(results, &mut trees, i));
  }
  fn grow_tree<'a>(
    results: &'a [ProcessedQueuedFile],
    trees: &mut Vec<Option<(PackJsonRootEntry, FlatDecisionTree)>>,
    index: usize
  ) -> DecisionTree<'a> {
    let Some((_json, tree)) = trees.get_mut(index).unwrap().take() else {
      unreachable!("partition_import_groups: attempt to remove already processed tree from pending set");
    };
    let new_tree = DecisionTree {
      description: tree.description,
      options: tree.options.into_iter().map(|option| {
        DecisionTreeEntry {
          description: option.description,
          files: option.files.into_iter().map(|i| &results[i]).collect(),
          subtrees: option.subtrees.into_iter().map(|i| grow_tree(results, trees, i)).collect()
        }
      }).collect()
    };
    new_tree
  }
  
  let remaining = trees.iter().filter(|x| x.is_some()).count();
  if remaining > 0 {
    unreachable!("partition_import_groups: found some unreachable tree segments")
  }
  
  // package any leftover loose files as its own tree
  if loose_files.len() > 0 {
    rooted_trees.push(DecisionTree {
      description: "content pack".to_string(),
      options: vec![DecisionTreeEntry {
        description: "loose files".to_string(),
        files: loose_files,
        subtrees: vec![]
      }]
    });
  }
  
  Ok(rooted_trees)
}















