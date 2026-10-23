use std::collections::HashMap;
use std::sync::Arc;

use isai_engine::effects::{Effect, EffectKind, Meters};
use isai_engine::instruments::{Instrument, InstrumentKind};
use isai_engine::plugin::{PluginEffect, PluginInstrument, PluginRef};
use isai_engine::{Engine, EngineConfig};
use isai_settings::Settings;

/// The running engine, or the reason it isn't running.
///
/// The app keeps working without audio (e.g. no output device plugged in) and
/// shows the error instead of quitting.
#[derive(Default)]
pub struct Audio {
    pub engine: Option<Engine>,
    pub error: Option<String>,
    config: Option<EngineConfig>,
    /// Effects the running engine has state for, by insert id, with their live
    /// meters (and which loaded plugin, for plugin inserts).
    effects: HashMap<u64, (EffectKind, Arc<Meters>, usize)>,
    /// Instruments likewise, by instrument id.
    instruments: HashMap<u64, (InstrumentKind, usize)>,
}

impl Audio {
    pub fn config_for(settings: &Settings) -> EngineConfig {
        EngineConfig {
            device_id: settings.audio.device_id.clone(),
            buffer_size: Some(settings.audio.buffer_size),
        }
    }

    /// Starts the engine with `config`, or restarts it if the config changed.
    /// Returns true when a new engine was started.
    pub fn ensure(&mut self, config: EngineConfig) -> bool {
        let same = self.config.as_ref().is_some_and(|c| {
            c.device_id == config.device_id && c.buffer_size == config.buffer_size
        });
        if same && (self.engine.is_some() || self.error.is_some()) {
            return false;
        }
        // Stop the old stream before opening the device again.
        self.engine = None;
        self.effects.clear();
        self.instruments.clear();
        self.config = Some(config.clone());
        match Engine::start(&config) {
            Ok(engine) => {
                self.engine = Some(engine);
                self.error = None;
                true
            }
            Err(err) => {
                self.error = Some(format!("Audio is unavailable: {err:#}"));
                false
            }
        }
    }

    /// Makes the engine's effects match `wanted` (insert id, kind, plugin):
    /// builds new ones here, off the audio thread, and drops ones no longer used.
    pub fn sync_effects(&mut self, wanted: &[(u64, EffectKind, Option<PluginRef>)]) {
        let Some(engine) = &mut self.engine else {
            return;
        };
        let rate = engine.info().sample_rate;
        self.effects.retain(|id, (kind, _, plugin)| {
            let keep = wanted
                .iter()
                .any(|(w, k, p)| w == id && k == kind && plugin_id(p) == *plugin);
            if !keep {
                engine.remove_effect(*id);
            }
            keep
        });
        for (id, kind, plugin) in wanted {
            if let std::collections::hash_map::Entry::Vacant(slot) = self.effects.entry(*id) {
                let meters = Arc::new(Meters::default());
                let effect = match (kind, plugin) {
                    (EffectKind::Plugin, Some(p)) => {
                        Effect::Plugin(PluginEffect::new(Some(p.clone()), false))
                    }
                    _ => Effect::new(*kind, rate),
                };
                engine.add_effect(*id, Box::new(effect), Some(meters.clone()));
                slot.insert((*kind, meters, plugin_id(plugin)));
            }
        }
    }

    /// Live readings of a running effect.
    pub fn meters(&self, id: u64) -> Option<Arc<Meters>> {
        self.effects.get(&id).map(|(_, m, _)| m.clone())
    }

    /// Makes the engine's instruments match `wanted` (id, kind), like `sync_effects`.
    pub fn sync_instruments(&mut self, wanted: &[(u64, InstrumentKind, Option<PluginRef>)]) {
        let Some(engine) = &mut self.engine else {
            return;
        };
        self.instruments.retain(|id, (kind, plugin)| {
            let keep = wanted
                .iter()
                .any(|(w, k, p)| w == id && k == kind && plugin_id(p) == *plugin);
            if !keep {
                engine.remove_instrument(*id);
            }
            keep
        });
        for (id, kind, plugin) in wanted {
            if let std::collections::hash_map::Entry::Vacant(slot) = self.instruments.entry(*id) {
                let instrument = match (kind, plugin) {
                    (InstrumentKind::Plugin, Some(p)) => {
                        Instrument::Plugin(PluginInstrument::new(Some(p.clone()), false))
                    }
                    _ => Instrument::new(*kind),
                };
                engine.add_instrument(*id, Box::new(instrument));
                slot.insert((*kind, plugin_id(plugin)));
            }
        }
    }

    /// Tries the same config again, e.g. after the user plugs in a device.
    pub fn retry(&mut self) -> bool {
        let config = self.config.take().unwrap_or_default();
        self.error = None;
        self.ensure(config)
    }
}

/// Which loaded plugin a handle points at (0 for none), to notice a reload.
fn plugin_id(plugin: &Option<PluginRef>) -> usize {
    plugin
        .as_ref()
        .map_or(0, |p| std::sync::Arc::as_ptr(&p.0) as *const () as usize)
}
