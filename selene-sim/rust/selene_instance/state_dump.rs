use super::SeleneInstance;
use anyhow::Result;
//use selene_core::encoder::StreamWritable;

impl SeleneInstance {
    /// Create a new file in the artifact directory, with a name based on the
    /// provided message (and with a unique integer suffix), and have the simulator
    /// write to it. Then write the chosen filename the out_encoder so that the
    /// result handler can know where to find the file corresponding to that part
    /// of the stream.
    pub fn dump_state(&mut self, message: &str, _qubits: &[u64]) -> Result<()> {
        // Create a new file in the artifact directory
        let filename_slug = message
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect::<String>()
            .trim_matches('_')
            .to_string();
        let path = (1..)
            .map(|i| {
                self.config
                    .artifact_dir
                    .join(format!("{}_{}.state", filename_slug, i))
            })
            .find(|f| !f.exists())
            .unwrap();
        self.emulator.dump_quantum_state(&path, _qubits)?;
        self.out_encoder.begin_message(self.time_cursor)?;
        self.out_encoder.write(message)?;
        self.out_encoder.write(path.to_str().unwrap())?;
        self.out_encoder.end_message()?;
        Ok(())
    }
}
