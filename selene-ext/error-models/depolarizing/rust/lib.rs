use anyhow::{Result, anyhow};
use clap::Parser;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use selene_core::error_model::interface::ErrorModelInterfaceFactory;
use selene_core::error_model::{BatchResult, ErrorModelInterface};
use selene_core::export_error_model_plugin;
use selene_core::runtime::{BatchOperation, Operation};
use selene_core::simulator::{Simulator, SimulatorInterface};
use selene_core::utils::MetricValue;
use std::ffi::OsStr;

#[derive(Parser, Debug)]
struct Params {
    /// The probability of a single-qubit gate error
    #[arg(long)]
    p_1q: f64,
    /// The probability of a two-qubit gate error
    #[arg(long)]
    p_2q: f64,
    /// The probability of a measurement error
    #[arg(long)]
    p_meas: f64,
    /// The probability of an initialization error
    #[arg(long)]
    p_init: f64,
}
#[derive(Default)]
struct Stats {
    gate_count_1q: u64,
    err_count_1q_x: u64,
    err_count_1q_y: u64,
    err_count_1q_z: u64,

    gate_count_2q: u64,
    err_count_2q_ix: u64,
    err_count_2q_iy: u64,
    err_count_2q_iz: u64,
    err_count_2q_xi: u64,
    err_count_2q_xx: u64,
    err_count_2q_xy: u64,
    err_count_2q_xz: u64,
    err_count_2q_yi: u64,
    err_count_2q_yx: u64,
    err_count_2q_yy: u64,
    err_count_2q_yz: u64,
    err_count_2q_zi: u64,
    err_count_2q_zx: u64,
    err_count_2q_zy: u64,
    err_count_2q_zz: u64,
    measure_count: u64,
    measure_errors: u64,
    init_count: u64,
    init_errors: u64,
}
pub enum ErrorType {
    I,
    X,
    Y,
    Z,
}

pub struct DepolarizingErrorModel {
    n_qubits: u64,
    rng: Pcg64Mcg,
    simulator: Simulator,
    error_params: Params,
    stats: Stats,
}

impl DepolarizingErrorModel {
    pub fn apply_error(&mut self, qubit: u64, error: ErrorType) -> Result<()> {
        match error {
            ErrorType::I => (),
            ErrorType::X => {
                self.simulator.rxy(qubit, std::f64::consts::PI, 0.0)?;
            }
            ErrorType::Y => {
                self.simulator
                    .rxy(qubit, std::f64::consts::PI, std::f64::consts::PI / 2.0)?;
            }
            ErrorType::Z => {
                self.simulator.rz(qubit, std::f64::consts::PI)?;
            }
        }
        Ok(())
    }
    fn maybe_apply_1q_error(&mut self, q0: u64) -> Result<()> {
        // validate arg
        if q0 >= self.n_qubits {
            return Err(anyhow!(
                "Error: q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ));
        }
        // generate error to apply
        let random_float = self.rng.random::<f64>();
        let error = if random_float > self.error_params.p_1q {
            ErrorType::I
        } else {
            let selection = (random_float * 3.0 / self.error_params.p_1q) as u64;
            match selection {
                0 => ErrorType::X,
                1 => ErrorType::Y,
                _ => ErrorType::Z,
            }
        };
        // update statistics
        self.stats.gate_count_1q += 1;
        match error {
            ErrorType::I => (),
            ErrorType::X => self.stats.err_count_1q_x += 1,
            ErrorType::Y => self.stats.err_count_1q_y += 1,
            ErrorType::Z => self.stats.err_count_1q_z += 1,
        }
        // apply error
        self.apply_error(q0, error)?;
        Ok(())
    }
    fn maybe_apply_2q_error(&mut self, q0: u64, q1: u64) -> Result<()> {
        // validate arg
        if q0 >= self.n_qubits || q1 >= self.n_qubits {
            return Err(anyhow!(
                "Error: q0 and q1 must be less than the number of qubits ({}).",
                self.n_qubits
            ));
        }
        // generate error to apply
        let random_float = self.rng.random::<f64>();
        let (error0, error1) = if random_float > self.error_params.p_2q {
            (ErrorType::I, ErrorType::I)
        } else {
            let selection = (random_float * 15.0 / self.error_params.p_2q) as u64;
            match selection {
                0 => (ErrorType::I, ErrorType::X),
                1 => (ErrorType::I, ErrorType::Y),
                2 => (ErrorType::I, ErrorType::Z),
                3 => (ErrorType::X, ErrorType::I),
                4 => (ErrorType::X, ErrorType::X),
                5 => (ErrorType::X, ErrorType::Y),
                6 => (ErrorType::X, ErrorType::Z),
                7 => (ErrorType::Y, ErrorType::I),
                8 => (ErrorType::Y, ErrorType::X),
                9 => (ErrorType::Y, ErrorType::Y),
                10 => (ErrorType::Y, ErrorType::Z),
                11 => (ErrorType::Z, ErrorType::I),
                12 => (ErrorType::Z, ErrorType::X),
                13 => (ErrorType::Z, ErrorType::Y),
                _ => (ErrorType::Z, ErrorType::Z),
            }
        };
        // update statistics
        self.stats.gate_count_2q += 1;
        match (&error0, &error1) {
            (ErrorType::I, ErrorType::I) => (),
            (ErrorType::I, ErrorType::X) => self.stats.err_count_2q_ix += 1,
            (ErrorType::I, ErrorType::Y) => self.stats.err_count_2q_iy += 1,
            (ErrorType::I, ErrorType::Z) => self.stats.err_count_2q_iz += 1,
            (ErrorType::X, ErrorType::I) => self.stats.err_count_2q_xi += 1,
            (ErrorType::X, ErrorType::X) => self.stats.err_count_2q_xx += 1,
            (ErrorType::X, ErrorType::Y) => self.stats.err_count_2q_xy += 1,
            (ErrorType::X, ErrorType::Z) => self.stats.err_count_2q_xz += 1,
            (ErrorType::Y, ErrorType::I) => self.stats.err_count_2q_yi += 1,
            (ErrorType::Y, ErrorType::X) => self.stats.err_count_2q_yx += 1,
            (ErrorType::Y, ErrorType::Y) => self.stats.err_count_2q_yy += 1,
            (ErrorType::Y, ErrorType::Z) => self.stats.err_count_2q_yz += 1,
            (ErrorType::Z, ErrorType::I) => self.stats.err_count_2q_zi += 1,
            (ErrorType::Z, ErrorType::X) => self.stats.err_count_2q_zx += 1,
            (ErrorType::Z, ErrorType::Y) => self.stats.err_count_2q_zy += 1,
            (ErrorType::Z, ErrorType::Z) => self.stats.err_count_2q_zz += 1,
        }
        // apply error
        self.apply_error(q0, error0)?;
        self.apply_error(q1, error1)?;
        Ok(())
    }
    fn maybe_flip_measurement(&mut self, _qubit: u64, result: bool) -> bool {
        let val = self.rng.random::<f64>();
        let flip = val < self.error_params.p_meas;
        self.stats.measure_count += 1;
        if flip {
            self.stats.measure_errors += 1;
            !result
        } else {
            result
        }
    }
    fn maybe_flip_on_init(&mut self, qubit: u64) -> Result<()> {
        self.stats.init_count += 1;
        let val = self.rng.random::<f64>();
        if val < self.error_params.p_init {
            self.stats.init_errors += 1;
            self.apply_error(qubit, ErrorType::X)?;
        }
        Ok(())
    }
}

impl ErrorModelInterface for DepolarizingErrorModel {
    fn shot_start(&mut self, shot_id: u64, seed: u64, simulator_seed: u64) -> Result<()> {
        self.rng = Pcg64Mcg::seed_from_u64(seed);
        self.simulator.shot_start(shot_id, simulator_seed)?;
        self.stats = Stats::default();
        Ok(())
    }
    fn shot_end(&mut self) -> Result<()> {
        self.simulator.shot_end()?;
        Ok(())
    }

    fn exit(&mut self) -> Result<()> {
        Ok(())
    }

    fn handle_operations(&mut self, operations: BatchOperation) -> Result<BatchResult> {
        let mut results = BatchResult::default();
        for op in operations {
            match op {
                Operation::RXYGate {
                    qubit_id,
                    theta,
                    phi,
                } => {
                    self.maybe_apply_1q_error(qubit_id)?;
                    self.simulator.rxy(qubit_id, theta, phi)?;
                }
                Operation::RZGate { qubit_id, theta } => {
                    self.maybe_apply_1q_error(qubit_id)?;
                    self.simulator.rz(qubit_id, theta)?;
                }
                Operation::RZZGate {
                    qubit_id_1,
                    qubit_id_2,
                    theta,
                } => {
                    self.maybe_apply_2q_error(qubit_id_1, qubit_id_2)?;
                    self.simulator.rzz(qubit_id_1, qubit_id_2, theta)?;
                }
                Operation::Measure {
                    qubit_id,
                    result_id,
                } => {
                    let measurement = self.simulator.measure(qubit_id)?;
                    let modified_measurement = self.maybe_flip_measurement(qubit_id, measurement);
                    results.set_bool_result(result_id, modified_measurement);
                }
                Operation::MeasureLeaked {
                    qubit_id,
                    result_id,
                } => {
                    // We aren't modelling leakage so this is the same as a normal measurement,
                    // except we set the u64 future as 0 or 1 (leakage would include higher values)
                    let measurement = self.simulator.measure(qubit_id)?;
                    let modified_measurement = self.maybe_flip_measurement(qubit_id, measurement);
                    results.set_u64_result(result_id, if modified_measurement { 1 } else { 0 });
                }
                Operation::Reset { qubit_id } => {
                    self.simulator.reset(qubit_id)?;
                    self.maybe_flip_on_init(qubit_id)?;
                }
                Operation::Custom { .. } => {
                    // Passively ignore custom operations
                }
            }
        }
        Ok(results)
    }

    fn get_metric(&mut self, nth_metric: u8) -> Result<Option<(String, MetricValue)>> {
        match nth_metric {
            0 => Ok(Some((
                "gates_1q".to_string(),
                MetricValue::U64(self.stats.gate_count_1q),
            ))),
            1 => Ok(Some((
                "errors_1q_x".to_string(),
                MetricValue::U64(self.stats.err_count_1q_x),
            ))),
            2 => Ok(Some((
                "errors_1q_y".to_string(),
                MetricValue::U64(self.stats.err_count_1q_y),
            ))),
            3 => Ok(Some((
                "errors_1q_z".to_string(),
                MetricValue::U64(self.stats.err_count_1q_z),
            ))),
            4 => Ok(Some((
                "gates_2q".to_string(),
                MetricValue::U64(self.stats.gate_count_2q),
            ))),
            5 => Ok(Some((
                "errors_2q_ix".to_string(),
                MetricValue::U64(self.stats.err_count_2q_ix),
            ))),
            6 => Ok(Some((
                "errors_2q_iy".to_string(),
                MetricValue::U64(self.stats.err_count_2q_iy),
            ))),
            7 => Ok(Some((
                "errors_2q_iz".to_string(),
                MetricValue::U64(self.stats.err_count_2q_iz),
            ))),
            8 => Ok(Some((
                "errors_2q_xi".to_string(),
                MetricValue::U64(self.stats.err_count_2q_xi),
            ))),
            9 => Ok(Some((
                "errors_2q_xx".to_string(),
                MetricValue::U64(self.stats.err_count_2q_xx),
            ))),
            10 => Ok(Some((
                "errors_2q_xy".to_string(),
                MetricValue::U64(self.stats.err_count_2q_xy),
            ))),
            11 => Ok(Some((
                "errors_2q_xz".to_string(),
                MetricValue::U64(self.stats.err_count_2q_xz),
            ))),
            12 => Ok(Some((
                "errors_2q_yi".to_string(),
                MetricValue::U64(self.stats.err_count_2q_yi),
            ))),
            13 => Ok(Some((
                "errors_2q_yx".to_string(),
                MetricValue::U64(self.stats.err_count_2q_yx),
            ))),
            14 => Ok(Some((
                "errors_2q_yy".to_string(),
                MetricValue::U64(self.stats.err_count_2q_yy),
            ))),
            15 => Ok(Some((
                "errors_2q_yz".to_string(),
                MetricValue::U64(self.stats.err_count_2q_yz),
            ))),
            16 => Ok(Some((
                "errors_2q_zi".to_string(),
                MetricValue::U64(self.stats.err_count_2q_zi),
            ))),
            17 => Ok(Some((
                "errors_2q_zx".to_string(),
                MetricValue::U64(self.stats.err_count_2q_zx),
            ))),
            18 => Ok(Some((
                "errors_2q_zy".to_string(),
                MetricValue::U64(self.stats.err_count_2q_zy),
            ))),
            19 => Ok(Some((
                "errors_2q_zz".to_string(),
                MetricValue::U64(self.stats.err_count_2q_zz),
            ))),
            20 => Ok(Some((
                "measurements".to_string(),
                MetricValue::U64(self.stats.measure_count),
            ))),
            21 => Ok(Some((
                "measurement_errors".to_string(),
                MetricValue::U64(self.stats.measure_errors),
            ))),
            22 => Ok(Some((
                "inits".to_string(),
                MetricValue::U64(self.stats.init_count),
            ))),
            23 => Ok(Some((
                "init_errors".to_string(),
                MetricValue::U64(self.stats.init_errors),
            ))),
            _ => Ok(None),
        }
    }
    fn get_simulator_metric(&mut self, nth_metric: u8) -> Result<Option<(String, MetricValue)>> {
        self.simulator.get_metric(nth_metric)
    }
}

#[derive(Default)]
pub struct DepolarizingErrorModelFactory;

impl ErrorModelInterfaceFactory for DepolarizingErrorModelFactory {
    type Interface = DepolarizingErrorModel;

    fn init(
        self: std::sync::Arc<Self>,
        n_qubits: u64,
        error_model_args: &[impl AsRef<str>],
        simulator_path: &impl AsRef<OsStr>,
        simulator_args: &[impl AsRef<str>],
    ) -> Result<Box<Self::Interface>> {
        match Params::try_parse_from(error_model_args.iter().map(|s| s.as_ref())) {
            Err(e) => Err(anyhow!(
                "Error parsing arguments to depolarizing error model plugin: {}",
                e
            )),
            Ok(params) => {
                let simulator =
                    Simulator::load_from_file(simulator_path, n_qubits, simulator_args)?;
                Ok(Box::new(DepolarizingErrorModel {
                    n_qubits,
                    rng: Pcg64Mcg::seed_from_u64(0),
                    simulator,
                    error_params: params,
                    stats: Stats::default(),
                }))
            }
        }
    }
}

export_error_model_plugin!(crate::DepolarizingErrorModelFactory);
