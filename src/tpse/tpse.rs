use std::collections::HashMap;

use crate::tpse::{Background, File, MiscTPSEValue, TouchControlConfig};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::de::{DeserializeOwned, Error};
use serde::ser::Error as SerError;

/// The root TPSE type
/// Essentially a schema for a key-value store
#[serde_with::skip_serializing_none]
#[serde_with::serde_as]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TPSE {
  #[serde(rename = "sfxEnabled")]
  pub sfx_enabled: Option<bool>,
  #[serde(rename = "musicEnabled")]
  pub music_enabled: Option<bool>,
  #[serde(rename = "musicGraphEnabled")]
  pub music_graph_enabled: Option<bool>,
  #[serde(rename = "disableVanillaMusic")]
  pub disable_vanilla_music: Option<bool>,
  #[serde(rename = "enableMissingMusicPatch")]
  pub enable_missing_music_patch: Option<bool>,
  #[serde(rename = "enableOSD")]
  pub enable_osd: Option<bool>,
  #[serde(rename = "bgEnabled")]
  pub bg_enabled: Option<bool>,
  #[serde(rename = "animatedBgEnabled")]
  pub animated_bg_enabled: Option<bool>,
  #[serde(rename = "enableTouchControls")]
  pub enable_touch_controls: Option<bool>,
  #[serde(rename = "enableEmoteTab")]
  pub enable_emote_tab: Option<bool>,
  #[serde(rename = "transparentBgEnabled")]
  pub transparent_bg_enabled: Option<bool>,
  #[serde(rename = "opaqueTransparentBackground")]
  pub opaque_transparent_background: Option<bool>,
  #[serde(rename = "openDevtoolsOnStart")]
  pub open_devtools_on_start: Option<bool>,
  #[serde(rename = "tetrioPlusEnabled")]
  pub tetrio_plus_enabled: Option<bool>,
  #[serde(rename = "hideTetrioPlusOnStartup")]
  pub hide_tetrio_plus_on_startup: Option<bool>,
  #[serde(rename = "allowURLPackLoader")]
  pub allow_url_pack_loader: Option<bool>,
  #[serde(rename = "whitelistedLoaderDomains")]
  pub whitelisted_loader_domains: Option<Vec<String>>,
  #[serde(rename = "enableCustomCss")]
  pub enable_custom_css: Option<bool>,
  #[serde(rename = "customCss")]
  pub custom_css: Option<String>,
  #[serde(rename = "enableAllSongTweaker")]
  pub enable_all_song_tweaker: Option<bool>,
  #[serde(rename = "showLegacyOptions")]
  pub show_legacy_options: Option<bool>,
  #[serde(rename = "bypassBootstrapper")]
  pub bypass_bootstrapper: Option<bool>,
  #[serde(rename = "enableCustomMaps")]
  pub enable_custom_maps: Option<bool>,
  #[serde(rename = "advancedSkinLoading")]
  pub advanced_skin_loading: Option<bool>,
  #[serde(rename = "forceNearestScaling")]
  pub force_nearest_scaling: Option<bool>,
  #[serde(rename = "windowTitleStatus")]
  pub window_title_status: Option<bool>,
  #[serde(rename = "musicGraphBackground")]
  pub music_graph_background: Option<bool>,
  pub board: Option<File>,
  #[serde(rename = "winterCompatEnabled")]
  pub winter_compat_enabled: Option<bool>,
  pub queue: Option<File>,
  pub grid: Option<File>,
  pub particle_beam: Option<File>,
  pub particle_beams_beam: Option<File>,
  pub particle_bigbox: Option<File>,
  pub particle_box: Option<File>,
  pub particle_chip: Option<File>,
  pub particle_chirp: Option<File>,
  pub particle_dust: Option<File>,
  pub particle_fbox: Option<File>,
  pub particle_fire: Option<File>,
  pub particle_particle: Option<File>,
  pub particle_smoke: Option<File>,
  pub particle_star: Option<File>,
  pub particle_flake: Option<File>,
  pub rank_d: Option<File>,
  pub rank_dplus: Option<File>,
  pub rank_cminus: Option<File>,
  pub rank_c: Option<File>,
  pub rank_cplus: Option<File>,
  pub rank_bminus: Option<File>,
  pub rank_b: Option<File>,
  pub rank_bplus: Option<File>,
  pub rank_aminus: Option<File>,
  pub rank_a: Option<File>,
  pub rank_aplus: Option<File>,
  pub rank_sminus: Option<File>,
  pub rank_s: Option<File>,
  pub rank_splus: Option<File>,
  pub rank_ss: Option<File>,
  pub rank_u: Option<File>,
  pub rank_x: Option<File>,
  pub rank_z: Option<File>,
  pub skin: Option<File>,
  pub ghost: Option<File>,
  #[serde(rename = "skinAnim")]
  pub skin_anim: Option<File>,
  #[serde(rename = "ghostAnim")]
  pub ghost_anim: Option<File>,
  #[serde(rename = "skinAnimMeta")]
  pub skin_anim_meta: Option<AnimMeta>,
  #[serde(rename = "ghostAnimMeta")]
  pub ghost_anim_meta: Option<AnimMeta>,
  #[serde(rename = "customSoundAtlas")]
  pub custom_sound_atlas: Option<HashMap<String, (f64, f64)>>,
  pub backgrounds: Option<Vec<Background>>,
  #[serde(rename = "animatedBackground")]
  pub animated_background: Option<AnimatedBackground>,
  pub music: Option<Vec<Song>>, // todo
  #[serde(rename = "musicGraph")]
  pub music_graph: Option<String>, // todo
  #[serde(rename = "touchControlConfig")]
  #[serde(deserialize_with = "deserialize_as_string", serialize_with = "serialize_as_string")]
  pub touch_control_config: Option<TouchControlConfig>,
  /// Other TPSE keys
  /// These should mainly be files for background and music IDs
  #[serde(flatten)]
  pub other: HashMap<String, MiscTPSEValue>
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Song {
  id: String,
  filename: String,
  #[serde(rename = "override")]
  song_override: Option<String>,
  metadata: SongMetadata
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SongMetadata {
  name: String,
  jpname: String,
  artist: String,
  jpartist: String,
  genre: SongGenre,
  source: String,
  #[serde(rename = "loop")]
  song_loop: bool,
  #[serde(rename = "loopStart")]
  loop_start: u32,
  #[serde(rename = "loopLength")]
  loop_length: u32
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SongGenre {
  Interface,
  Disabled,
  Override,
  Calm,
  Battle
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AnimMeta {
  /// The number of frames the animation lasts for
  pub frames: u32,
  /// The delay between frames, in game frames (e.g. 30 = 2fps)
  pub delay: u32
}

fn serialize_as_string<T, S>(value: &T, ser: S) -> Result<S::Ok, S::Error> where T: Serialize, S: Serializer {
  match serde_json::to_string(value) {
    Ok(string) => Ok(ser.serialize_str(&string)?),
    Err(err) => Err(S::Error::custom(err))
  }
}

fn deserialize_as_string<'a, T, D>(de: D) -> Result<T, D::Error> where T: DeserializeOwned, D: Deserializer<'a> {
  serde_json::from_str(&String::deserialize(de)?).map_err(D::Error::custom)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AnimatedBackground {
  pub id: String,
  pub filename: String
}

impl TPSE {
  pub fn merge(&mut self, mut other: TPSE) {
    self.sfx_enabled = other.sfx_enabled.or(self.sfx_enabled.take());
    self.music_enabled = other.music_enabled.or(self.music_enabled.take());
    self.music_graph_enabled = other.music_graph_enabled.or(self.music_graph_enabled.take());
    self.disable_vanilla_music = other.disable_vanilla_music.or(self.disable_vanilla_music.take());
    self.enable_missing_music_patch = other.enable_missing_music_patch.or(self.enable_missing_music_patch.take());
    self.enable_osd = other.enable_osd.or(self.enable_osd.take());
    self.bg_enabled = other.bg_enabled.or(self.bg_enabled.take());
    self.animated_bg_enabled = other.animated_bg_enabled.or(self.animated_bg_enabled.take());
    self.enable_touch_controls = other.enable_touch_controls.or(self.enable_touch_controls.take());
    self.enable_emote_tab = other.enable_emote_tab.or(self.enable_emote_tab.take());
    self.transparent_bg_enabled = other.transparent_bg_enabled.or(self.transparent_bg_enabled.take());
    self.opaque_transparent_background = other.opaque_transparent_background.or(self.opaque_transparent_background.take());
    self.open_devtools_on_start = other.open_devtools_on_start.or(self.open_devtools_on_start.take());
    self.tetrio_plus_enabled = other.tetrio_plus_enabled.or(self.tetrio_plus_enabled.take());
    self.hide_tetrio_plus_on_startup = other.hide_tetrio_plus_on_startup.or(self.hide_tetrio_plus_on_startup.take());
    self.allow_url_pack_loader = other.allow_url_pack_loader.or(self.allow_url_pack_loader.take());
    self.whitelisted_loader_domains = other.whitelisted_loader_domains.or(self.whitelisted_loader_domains.take());
    self.enable_custom_css = other.enable_custom_css.or(self.enable_custom_css.take());
    self.custom_css = other.custom_css.or(self.custom_css.take());
    self.enable_all_song_tweaker = other.enable_all_song_tweaker.or(self.enable_all_song_tweaker.take());
    self.show_legacy_options = other.show_legacy_options.or(self.show_legacy_options.take());
    self.bypass_bootstrapper = other.bypass_bootstrapper.or(self.bypass_bootstrapper.take());
    self.enable_custom_maps = other.enable_custom_maps.or(self.enable_custom_maps.take());
    self.advanced_skin_loading = other.advanced_skin_loading.or(self.advanced_skin_loading.take());
    self.force_nearest_scaling = other.force_nearest_scaling.or(self.force_nearest_scaling.take());
    self.window_title_status = other.window_title_status.or(self.window_title_status.take());
    self.music_graph_background = other.music_graph_background.or(self.music_graph_background.take());
    self.board = other.board.or(self.board.take());
    self.winter_compat_enabled = other.winter_compat_enabled.or(self.winter_compat_enabled.take());
    self.queue = other.queue.or(self.queue.take());
    self.grid = other.grid.or(self.grid.take());
    self.particle_beam = other.particle_beam.or(self.particle_beam.take());
    self.particle_beams_beam = other.particle_beams_beam.or(self.particle_beams_beam.take());
    self.particle_bigbox = other.particle_bigbox.or(self.particle_bigbox.take());
    self.particle_box = other.particle_box.or(self.particle_box.take());
    self.particle_chip = other.particle_chip.or(self.particle_chip.take());
    self.particle_chirp = other.particle_chirp.or(self.particle_chirp.take());
    self.particle_dust = other.particle_dust.or(self.particle_dust.take());
    self.particle_fbox = other.particle_fbox.or(self.particle_fbox.take());
    self.particle_fire = other.particle_fire.or(self.particle_fire.take());
    self.particle_particle = other.particle_particle.or(self.particle_particle.take());
    self.particle_smoke = other.particle_smoke.or(self.particle_smoke.take());
    self.particle_star = other.particle_star.or(self.particle_star.take());
    self.particle_flake = other.particle_flake.or(self.particle_flake.take());
    self.rank_d = other.rank_d.or(self.rank_d.take());
    self.rank_dplus = other.rank_dplus.or(self.rank_dplus.take());
    self.rank_cminus = other.rank_cminus.or(self.rank_cminus.take());
    self.rank_c = other.rank_c.or(self.rank_c.take());
    self.rank_cplus = other.rank_cplus.or(self.rank_cplus.take());
    self.rank_bminus = other.rank_bminus.or(self.rank_bminus.take());
    self.rank_b = other.rank_b.or(self.rank_b.take());
    self.rank_bplus = other.rank_bplus.or(self.rank_bplus.take());
    self.rank_aminus = other.rank_aminus.or(self.rank_aminus.take());
    self.rank_a = other.rank_a.or(self.rank_a.take());
    self.rank_aplus = other.rank_aplus.or(self.rank_aplus.take());
    self.rank_sminus = other.rank_sminus.or(self.rank_sminus.take());
    self.rank_s = other.rank_s.or(self.rank_s.take());
    self.rank_splus = other.rank_splus.or(self.rank_splus.take());
    self.rank_ss = other.rank_ss.or(self.rank_ss.take());
    self.rank_u = other.rank_u.or(self.rank_u.take());
    self.rank_x = other.rank_x.or(self.rank_x.take());
    self.rank_z = other.rank_z.or(self.rank_z.take());
    self.skin = other.skin.or(self.skin.take());
    self.ghost = other.ghost.or(self.ghost.take());
    self.skin_anim = other.skin_anim.or(self.skin_anim.take());
    self.ghost_anim = other.ghost_anim.or(self.ghost_anim.take());
    self.skin_anim_meta = other.skin_anim_meta.or(self.skin_anim_meta.take());
    self.ghost_anim_meta = other.ghost_anim_meta.or(self.ghost_anim_meta.take());
    self.custom_sound_atlas = other.custom_sound_atlas.or(self.custom_sound_atlas.take());
    match (self.backgrounds.is_some(), other.backgrounds.is_some()) {
      (true, true) => self.backgrounds.as_mut().unwrap().extend(other.backgrounds.unwrap()),
      (false, true) => self.backgrounds = other.backgrounds.take(),
      (_, false) => {}
    }
    self.animated_background = other.animated_background.or(self.animated_background.take());
    match (self.music.is_some(), other.music.is_some()) {
      (true, true) => self.music.as_mut().unwrap().extend(other.music.unwrap()),
      (false, true) => self.music = other.music.take(),
      (_, false) => {}
    }
    // todo: smarter merging
    self.music_graph = other.music_graph.or(self.music_graph.take());
    self.touch_control_config = other.touch_control_config.or(self.touch_control_config.take());
    self.other.extend(other.other.drain());
  }
}