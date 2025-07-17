use super::SeleneInstance;
use anyhow::Result;
use selene_core::encoder::StreamWritable;

impl SeleneInstance {
    /// Write a tagged boolean variable to the result stream.
    pub fn print<T: StreamWritable>(&mut self, tag: &str, value: T) -> Result<()> {
        self.out_encoder.begin_message(self.time_cursor)?;
        self.out_encoder.write(tag)?;
        self.out_encoder.write(value)?;
        self.out_encoder.end_message()?;
        Ok(())
    }

    pub fn print_panic(&mut self, message: &str, error_code: u32) -> Result<()> {
        let tag = if message.starts_with("EXIT:INT:") {
            message.to_string()
        } else {
            format!("EXIT:INT:{}", message)
        };
        let error_code_u64 = error_code as u64;
        self.out_encoder.begin_message(self.time_cursor)?;
        self.out_encoder.write(tag.as_str())?;
        self.out_encoder.write(error_code_u64)?;
        self.out_encoder.end_message()?;
        Ok(())
    }
    pub fn print_exit(&mut self, message: &str, error_code: u32) -> Result<()> {
        let tag = if message.starts_with("EXIT:INT:") {
            message.to_string()
        } else {
            format!("EXIT:INT:{}", message)
        };
        let error_code_u64 = error_code as u64;
        self.out_encoder.begin_message(self.time_cursor)?;
        self.out_encoder.write(tag.as_str())?;
        self.out_encoder.write(error_code_u64)?;
        self.out_encoder.end_message()?;
        Ok(())
    }
    pub fn print_shot_boundary(&mut self) -> Result<()> {
        self.print("USER:__SHOT_BOUNDARY__", 0u64)
    }
}
