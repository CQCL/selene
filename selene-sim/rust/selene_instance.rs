#![allow(static_mut_refs)]

use anyhow::Result;

pub mod configuration;
pub mod metadata;
pub mod print;
pub mod quantum;
pub mod rng;
pub mod state_dump;
use configuration::Configuration;

use crate::emulator::Emulator;
use crate::event_hooks::EventHook;
use crate::event_hooks::MultiEventHook;
use rand_pcg::Pcg32;
use selene_core::encoder::OutputStream;
use selene_core::error_model::ErrorModel;
use selene_core::error_model::ErrorModelInterface;
use selene_core::error_model::plugin::ErrorModelPluginInterface;
use selene_core::runtime::Runtime;
use selene_core::runtime::RuntimeInterface;
use selene_core::runtime::plugin::RuntimePluginInterface;

pub struct SeleneInstance {
    pub config: Configuration,
    pub emulator: Emulator,
    pub out_encoder: OutputStream,
    pub time_cursor: u64,
    pub shot_number: u64,
    pub prng: Option<Pcg32>,
}

impl SeleneInstance {
    pub fn new(config: Configuration) -> Result<Self> {
        let out_encoder = OutputStream::new(config.get_output_writer());
        let n_qubits = config.n_qubits;
        let error_model_plugin =
            ErrorModelPluginInterface::new_from_file(&config.error_model.file)?;
        let error_model = ErrorModel::new(
            error_model_plugin,
            n_qubits,
            config.error_model.args.as_ref(),
            &config.simulator.file,
            config.simulator.args.as_ref(),
        )?;

        let runtime_plugin = RuntimePluginInterface::new_from_file(&config.runtime.file)?;
        let runtime = Runtime::new(
            runtime_plugin,
            n_qubits,
            selene_core::time::Instant::default(),
            config.runtime.args.as_ref(),
        )?;

        // Set up the event hooks
        let mut event_hooks = MultiEventHook::default();
        if config.event_hooks.provide_metrics {
            event_hooks.add_hook(Box::new(
                crate::event_hooks::metrics::HighLevelMetrics::default(),
            ));
        }
        if config.event_hooks.provide_instruction_log {
            event_hooks.add_hook(Box::new(
                crate::event_hooks::instruction_log::InstructionLog::default(),
            ));
        }

        let emulator = Emulator {
            runtime,
            error_model,
            event_hooks,
        };

        let shot_offset = config.shots.offset;
        Ok(Self {
            config,
            emulator,
            out_encoder,
            time_cursor: 0,
            shot_number: shot_offset,
            prng: None,
        })
    }
    pub fn exit(&mut self) -> Result<()> {
        self.out_encoder.end_of_stream()?;
        self.out_encoder.flush()?;
        Ok(())
    }

    pub fn shot_start(&mut self, shot_index: u64) -> Result<()> {
        let shot_id = self.config.shots.offset + self.config.shots.increment * shot_index;
        let runtime_seed = self.config.runtime.seed + shot_id;
        let error_model_seed = self.config.error_model.seed + shot_id;
        let simulator_seed = self.config.simulator.seed + shot_id;
        self.shot_number = shot_id;
        self.emulator.runtime.shot_start(shot_id, runtime_seed)?;
        self.emulator.event_hooks.on_shot_start(shot_id);
        self.emulator
            .error_model
            .shot_start(shot_id, error_model_seed, simulator_seed)?;
        self.emulator.poke()?;
        Ok(())
    }
    pub fn shot_end(&mut self) -> Result<()> {
        self.write_metadata()?;
        if self.config.event_hooks.provide_metrics {
            self.write_metrics()?;
        }
        self.print_shot_boundary()?;
        self.emulator.event_hooks.on_shot_end();
        self.emulator.runtime.shot_end()?;
        self.emulator.error_model.shot_end()?;
        self.emulator.poke()?;
        Ok(())
    }
}
