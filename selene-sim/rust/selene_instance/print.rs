use super::SeleneInstance;
use anyhow::Result;
use selene_core::encoder::{OutputStream, StreamWritable};

pub fn print_directly_to_stream<T: StreamWritable>(
    out_encoder: &mut OutputStream,
    time_cursor: u64,
    tag: &str,
    value: T,
) -> Result<()> {
    // In some cases, e.g. the compiled user program, the exit namespace is already
    // encoded in the provided message. In others, e.g. runtime panics from components,
    // we need to prepend it.
    out_encoder.begin_message(time_cursor)?;
    out_encoder.write(tag)?;
    out_encoder.write(value)?;
    out_encoder.end_message()?;
    Ok(())
}

impl SeleneInstance {
    /// Write a tagged boolean variable to the result stream.
    pub fn print<T: StreamWritable>(&mut self, tag: &str, value: T) -> Result<()> {
        print_directly_to_stream(&mut self.out_encoder, self.time_cursor, tag, value)
    }
    pub fn print_panic(&mut self, message: &str, error_code: u32) -> Result<()> {
        // In some cases, e.g. the compiled user program, the exit namespace is already
        // encoded in the provided message. In others, e.g. runtime panics from components,
        // we need to prepend it.
        let tag = if message.starts_with("EXIT:INT:") {
            message.to_string()
        } else {
            format!("EXIT:INT:{}", message)
        };
        self.print(&tag, error_code as u64)
    }
    pub fn print_exit(&mut self, message: &str, error_code: u32) -> Result<()> {
        // At present, we handle exits in the same way as panics - the code
        // is used to communicate the semantics. This is a placeholder incase
        // we want to differentiate them in the future.
        self.print_panic(message, error_code)
    }
    pub fn fallible_print_panic(&mut self, message: &str, error_code: u32) {
        if self.print_panic(message, error_code).is_err() {
            // If printing the panic fails, there's not much we can do
            // to communicate the issue to the frontend, so we log it to stderr.
            eprintln!("Panic #{error_code}: {message}");
        }
    }
    pub fn print_shot_start(&mut self) -> Result<()> {
        self.print("SELENE:SHOT_START", self.shot_number)
    }
    pub fn print_shot_end(&mut self) -> Result<()> {
        self.print("SELENE:SHOT_END", self.shot_number)
    }
}
