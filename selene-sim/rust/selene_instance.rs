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
use rand_pcg::Pcg32;
use selene_core::encoder::OutputStream;
use selene_core::error_model::ErrorModelInterface;
use selene_core::runtime::RuntimeInterface;

pub struct SeleneInstance {
    pub config: Configuration,
    pub emulator: Emulator,
    pub out_encoder: OutputStream,
    pub time_cursor: u64,
    pub shot_number: u64,
    pub prng: Option<Pcg32>,
}

impl SeleneInstance {
    /// Create a new Selene simulator instance from the provided configuration,
    /// e.g. loaded from a config file.
    pub fn new(config: Configuration) -> Result<Self> {
        let mut out_encoder = OutputStream::new(config.get_output_writer());
        let emulator = match Emulator::from_configuration(&config) {
            Ok(emulator) => emulator,
            Err(e) => {
                // Note: with the current API design, the out_encoder is locally scoped and owned
                // by self, so if we simply return an error here there is no way to communicate that
                // error through the OutputStream. This can lead to to frontends having to
                // guess what went wrong during initialization.
                //
                // A future API design should consider allowing the output stream to be configured
                // outside of the config file and passed in.
                //
                // Avoiding an API break for now, we write the error to the stream here before returning.
                eprintln!("Failed to initialize SeleneInstance: {:?}", e);
                print::print_directly_to_stream(
                    &mut out_encoder,
                    0,
                    "EXIT:INT:Failed during initialization of emulator components",
                    190000u64,
                )?;
                return Err(e);
            }
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

    /// Upon termination of the simulator instance, we signal to the output stream
    /// that we are closing out gracefully, and flush all remaining data.
    pub fn exit(&mut self) -> Result<()> {
        self.out_encoder.end_of_stream()?;
        self.out_encoder.flush()?;
        Ok(())
    }

    /// Start a new shot, with a monatonically increasing shot index.
    /// This sets up the runtime and error model for the new shot and
    /// prepares the emulator to process operations for the shot.
    ///
    /// Although we operate on ranges of shots with an offset and increment,
    /// this is handled inside this function. As such, the shot_index parameter
    /// should start at 0 and increment by 1 for each new shot.
    pub fn shot_start(&mut self, shot_index: u64) -> Result<()> {
        // Establish the internal shot ID based on the offset and increment,
        // and use it for seeding the various components.
        let shot_id = self.config.shots.offset + self.config.shots.increment * shot_index;
        self.shot_number = shot_id;
        self.print_shot_start()?;
        let runtime_seed = self.config.runtime.seed + shot_id;
        let error_model_seed = self.config.error_model.seed + shot_id;
        let simulator_seed = self.config.simulator.seed + shot_id;

        // Now we fire off any shot start event hooks and prepare the
        // runtime and error model for the new shot.
        self.emulator.event_hooks.on_shot_start(shot_id);
        self.emulator.runtime.shot_start(shot_id, runtime_seed)?;
        self.emulator
            .error_model
            .shot_start(shot_id, error_model_seed, simulator_seed)?;
        // If the runtime wishes to perform any startup operations
        // before the main shot processing begins, we process them now.
        self.emulator.poke()?;
        Ok(())
    }

    /// End the current shot, flushing any remaining operations
    /// through the runtime and error model, writing out any
    /// stored metadata, and informing event hooks that the shot
    /// has ended.
    pub fn shot_end(&mut self) -> Result<()> {
        // Emit any metrics before plugins are invoked with shot_end.
        // This ensures that metrics reflect the state of the shot
        // up to this point.
        if self.config.event_hooks.provide_metrics {
            self.write_metrics()?;
        }
        // Tell the runtime that it's time to shut down.
        // This may flush additional operations as part of
        // the shutdown process.
        self.emulator.runtime.shot_end()?;
        // Handle any operations that resulted from the runtime's shot
        // end process
        self.emulator.poke()?;
        // Tell the error model, having already processed anything from
        // the current runtime shot, that the shot is ending. The error model
        // must also end the shot on its internal simulator.
        self.emulator.error_model.shot_end()?;
        // Finally, write out any stored metadata e.g. instruction logs
        // and inform event hooks that the shot has ended.
        self.write_metadata()?;
        self.emulator.event_hooks.on_shot_end();
        // Print shot boundary information so that the result stream
        // is properly delimited.
        self.print_shot_end()?;
        Ok(())
    }
}
