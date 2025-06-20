use anyhow::Result;

use selene_core::encoder::OutputStreamError;
use selene_core::error_model::ErrorModelInterface;
use selene_core::runtime::RuntimeInterface;
use selene_core::utils::MetricValue;

use crate::event_hooks::EventHook;
use crate::selene_instance::SeleneInstance;

impl SeleneInstance {
    fn write_metric(
        &mut self,
        metric_category: &str,
        metric_tag: &str,
        metric_value: MetricValue,
    ) -> Result<(), OutputStreamError> {
        let type_str = match metric_value {
            MetricValue::Bool(_) => "BOOL",
            MetricValue::I64(_) => "INT",
            MetricValue::U64(_) => "INT",
            MetricValue::F64(_) => "FLOAT",
        };
        let full_tag = format!("METRICS:{}:{}:{}", type_str, metric_category, metric_tag);
        self.out_encoder.begin_message(self.time_cursor)?;
        self.out_encoder.write(full_tag.as_str())?;
        match metric_value {
            MetricValue::Bool(v) => self.out_encoder.write(v)?,
            MetricValue::I64(v) => self.out_encoder.write(v)?,
            MetricValue::U64(v) => self.out_encoder.write(v)?,
            MetricValue::F64(v) => self.out_encoder.write(v)?,
        };
        self.out_encoder.end_message()?;
        Ok(())
    }
    pub fn write_metrics(&mut self) -> Result<()> {
        self.write_metric(
            "emulator",
            "shot_number",
            MetricValue::U64(self.shot_number),
        )?;
        // Write the metrics from the runtime plugin
        for nth_metric in 0u8..255u8 {
            let maybe_runtime_metric = self.emulator.runtime.get_metric(nth_metric)?;
            let Some((tag, value)) = maybe_runtime_metric else {
                break;
            };
            self.write_metric("runtime", &tag, value)?;
        }
        // Write the metrics from the error model plugin
        for nth_metric in 0u8..255u8 {
            let maybe_error_model_metric = self.emulator.error_model.get_metric(nth_metric)?;
            let Some((tag, value)) = maybe_error_model_metric else {
                break;
            };
            self.write_metric("error_model", &tag, value)?;
        }
        // Write the metrics from the simulator plugin
        for nth_metric in 0u8..255u8 {
            let maybe_simulator_metric =
                self.emulator.error_model.get_simulator_metric(nth_metric)?;
            let Some((tag, value)) = maybe_simulator_metric else {
                break;
            };
            self.write_metric("simulator", &tag, value)?;
        }
        Ok(())
    }
    pub fn write_metadata(&mut self) -> Result<()> {
        self.emulator
            .event_hooks
            .write(self.time_cursor, &mut self.out_encoder)?;
        Ok(())
    }
}
