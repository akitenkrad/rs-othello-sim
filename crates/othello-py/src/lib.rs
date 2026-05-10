//! Python bindings for `rs-othello-sim` (PyO3 / maturin).
//!
//! Exposes [`OthelloEnv`] and [`OthelloMultiEnv`] from `othello-rl` to Python.
//! Build with `maturin develop` (the default `cargo build` does not link Python).
//!
//! See `examples/python_smoke.py` for usage.

// PyO3 0.22 macros emit code that uses unsafe operations inside `unsafe fn`
// without an explicit `unsafe { ... }` block, which Rust 2024 flags via the
// `unsafe_op_in_unsafe_fn` lint. Also the macro-generated function signatures
// trigger `clippy::type_complexity` and `clippy::useless_conversion`. Suppress
// at the crate level until PyO3 catches up.
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(clippy::type_complexity)]
#![allow(clippy::useless_conversion)]

use ndarray::{Array1, Array3};
use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1, PyReadonlyArray3};
use othello_core::{BoardSize, Color};
use othello_player::{Player, player_spec::parse_player_spec};
use othello_rl::{
    Action, EnvConfig, MultiEnvConfig, Observation, ObservationType, OthelloEnv, OthelloMultiEnv,
    PrioritizedReplayBuffer, ReplayBuffer, RewardMode, Transition, UniformReplayBuffer,
};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

fn parse_color(s: &str) -> PyResult<Color> {
    match s.to_ascii_lowercase().as_str() {
        "black" | "b" => Ok(Color::Black),
        "white" | "w" => Ok(Color::White),
        other => Err(PyValueError::new_err(format!(
            "agent_color must be 'black' or 'white', got {other:?}"
        ))),
    }
}

fn parse_obs_type(s: &str) -> PyResult<ObservationType> {
    match s.to_ascii_lowercase().as_str() {
        "planes" => Ok(ObservationType::Planes),
        "flat" => Ok(ObservationType::Flat),
        "move_sequence" | "movesequence" | "seq" => Ok(ObservationType::MoveSequence),
        other => Err(PyValueError::new_err(format!(
            "observation_type must be 'planes' | 'flat' | 'move_sequence', got {other:?}"
        ))),
    }
}

fn parse_reward_mode(s: &str) -> PyResult<RewardMode> {
    match s.to_ascii_lowercase().as_str() {
        "sparse" => Ok(RewardMode::Sparse),
        "dense" => Ok(RewardMode::Dense),
        other => Err(PyValueError::new_err(format!(
            "reward_mode must be 'sparse' | 'dense', got {other:?}"
        ))),
    }
}

fn build_opponent(spec: &str, color: Color) -> PyResult<Box<dyn Player>> {
    let parsed = parse_player_spec(spec)
        .map_err(|e| PyValueError::new_err(format!("invalid opponent spec: {e}")))?;
    parsed
        .try_build_player(color)
        .map_err(|e| PyValueError::new_err(format!("cannot build opponent: {e}")))
}

fn observation_to_py<'py>(py: Python<'py>, obs: Observation) -> PyResult<Bound<'py, PyAny>> {
    Ok(match obs {
        Observation::Planes(a) => {
            let arr: Array3<f32> = a;
            arr.into_pyarray_bound(py).into_any()
        }
        Observation::Flat(a) => {
            let arr: Array1<f32> = a;
            arr.into_pyarray_bound(py).into_any()
        }
        Observation::MoveSequence(v) => {
            let arr: Array1<u32> = v.into_iter().map(|x| x as u32).collect();
            arr.into_pyarray_bound(py).into_any()
        }
    })
}

fn info_to_py<'py>(py: Python<'py>, info: &othello_rl::StepInfo) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new_bound(py);
    let mask: Array1<bool> = info.action_mask.clone();
    let mask_py: Bound<'py, PyArray1<bool>> = mask.into_pyarray_bound(py);
    dict.set_item("action_mask", mask_py)?;
    dict.set_item("legal_count", info.legal_count)?;
    dict.set_item("move_number", info.move_number)?;
    dict.set_item(
        "side_to_move",
        match info.side_to_move {
            Color::Black => "black",
            Color::White => "white",
        },
    )?;
    Ok(dict)
}

/// Gymnasium-style single-agent Othello environment.
#[pyclass(name = "OthelloEnv", module = "othello_sim")]
pub struct PyOthelloEnv {
    inner: OthelloEnv,
}

#[pymethods]
impl PyOthelloEnv {
    #[new]
    #[pyo3(signature = (
        board_size = 8,
        opponent = "random",
        observation_type = "planes",
        reward_mode = "sparse",
        seed = None,
        agent_color = "black",
        max_steps = None
    ))]
    pub fn new(
        board_size: u8,
        opponent: &str,
        observation_type: &str,
        reward_mode: &str,
        seed: Option<u64>,
        agent_color: &str,
        max_steps: Option<u32>,
    ) -> PyResult<Self> {
        let agent = parse_color(agent_color)?;
        let obs_ty = parse_obs_type(observation_type)?;
        let mode = parse_reward_mode(reward_mode)?;
        let opponent_color = agent.opponent();
        let opp = build_opponent(opponent, opponent_color)?;
        let cfg = EnvConfig {
            board_size: BoardSize::square(board_size),
            agent_color: agent,
            observation_type: obs_ty,
            reward_mode: mode,
            include_history_in_obs: false,
            max_steps,
        };
        let mut inner = OthelloEnv::new(cfg, opp);
        // 初期 reset を呼ぶ ( seed 指定があればそれを使う)
        inner.reset(seed);
        Ok(Self { inner })
    }

    /// `reset(seed=None)` -> `(obs, info)`.
    #[pyo3(signature = (seed = None))]
    pub fn reset<'py>(
        &mut self,
        py: Python<'py>,
        seed: Option<u64>,
    ) -> PyResult<(Bound<'py, PyAny>, Bound<'py, PyDict>)> {
        let (obs, info) = self.inner.reset(seed);
        let obs_py = observation_to_py(py, obs)?;
        let info_py = info_to_py(py, &info)?;
        Ok((obs_py, info_py))
    }

    /// `step(action)` -> `(obs, reward, terminated, truncated, info)`.
    pub fn step<'py>(
        &mut self,
        py: Python<'py>,
        action: u32,
    ) -> PyResult<(Bound<'py, PyAny>, f32, bool, bool, Bound<'py, PyDict>)> {
        let r = self
            .inner
            .step(Action(action))
            .map_err(|e| PyRuntimeError::new_err(format!("env step error: {e}")))?;
        let obs_py = observation_to_py(py, r.observation)?;
        let info_py = info_to_py(py, &r.info)?;
        Ok((obs_py, r.reward, r.terminated, r.truncated, info_py))
    }

    /// 合法 Action のリストを返す ( Python list of int)．
    pub fn legal_actions<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let actions = self.inner.legal_actions();
        let list = PyList::empty_bound(py);
        for a in actions {
            list.append(a.0)?;
        }
        Ok(list)
    }

    /// 合法手マスク ( numpy bool array)．
    pub fn action_mask<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<bool>> {
        self.inner.action_mask().into_pyarray_bound(py)
    }

    /// ASCII 描画．
    pub fn render(&self) -> String {
        self.inner.render()
    }

    /// `Action` 空間サイズ ( 整数)．
    #[getter]
    pub fn action_space_size(&self) -> u32 {
        Action::space_size(self.inner.config().board_size)
    }

    /// 盤面サイズ ( 整数)．
    #[getter]
    pub fn board_size(&self) -> u8 {
        self.inner.config().board_size.rows
    }
}

/// PettingZoo-style multi-agent Othello environment.
#[pyclass(name = "OthelloMultiEnv", module = "othello_sim")]
pub struct PyOthelloMultiEnv {
    inner: OthelloMultiEnv,
}

#[pymethods]
impl PyOthelloMultiEnv {
    #[new]
    #[pyo3(signature = (
        board_size = 8,
        observation_type = "planes",
        reward_mode = "sparse",
        max_steps = None
    ))]
    pub fn new(
        board_size: u8,
        observation_type: &str,
        reward_mode: &str,
        max_steps: Option<u32>,
    ) -> PyResult<Self> {
        let obs_ty = parse_obs_type(observation_type)?;
        let mode = parse_reward_mode(reward_mode)?;
        let cfg = MultiEnvConfig {
            board_size: BoardSize::square(board_size),
            observation_type: obs_ty,
            reward_mode: mode,
            max_steps,
        };
        Ok(Self {
            inner: OthelloMultiEnv::new(cfg),
        })
    }

    /// `reset(seed=None)` -> `dict[agent_id, obs]`．
    #[pyo3(signature = (seed = None))]
    pub fn reset<'py>(
        &mut self,
        py: Python<'py>,
        seed: Option<u64>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let obs_map = self.inner.reset(seed);
        let dict = PyDict::new_bound(py);
        for (k, v) in obs_map {
            dict.set_item(k, observation_to_py(py, v)?)?;
        }
        Ok(dict)
    }

    /// 現手番 agent ( "black" | "white")．
    pub fn current_agent(&self) -> String {
        self.inner.current_agent()
    }

    /// 全 agent の ID リスト．
    pub fn agents<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        let list = PyList::empty_bound(py);
        for a in self.inner.agents() {
            let _ = list.append(a);
        }
        list
    }

    /// 指定 agent の観測．
    pub fn observe<'py>(&self, py: Python<'py>, agent: &str) -> PyResult<Bound<'py, PyAny>> {
        observation_to_py(py, self.inner.observe(agent))
    }

    /// 指定 agent の合法手マスク．
    pub fn action_mask<'py>(&self, py: Python<'py>, agent: &str) -> Bound<'py, PyArray1<bool>> {
        self.inner.action_mask(agent).into_pyarray_bound(py)
    }

    /// `step(action)` -> `(observations, rewards, terminations, truncations, info)`．
    pub fn step<'py>(
        &mut self,
        py: Python<'py>,
        action: u32,
    ) -> PyResult<(
        Bound<'py, PyDict>,
        Bound<'py, PyDict>,
        Bound<'py, PyDict>,
        Bound<'py, PyDict>,
        Bound<'py, PyDict>,
    )> {
        let r = self
            .inner
            .step(Action(action))
            .map_err(|e| PyRuntimeError::new_err(format!("multi env step error: {e}")))?;
        let obs_d = PyDict::new_bound(py);
        for (k, v) in r.observations {
            obs_d.set_item(k, observation_to_py(py, v)?)?;
        }
        let rew_d = PyDict::new_bound(py);
        for (k, v) in r.rewards {
            rew_d.set_item(k, v)?;
        }
        let term_d = PyDict::new_bound(py);
        for (k, v) in r.terminations {
            term_d.set_item(k, v)?;
        }
        let trunc_d = PyDict::new_bound(py);
        for (k, v) in r.truncations {
            trunc_d.set_item(k, v)?;
        }
        let info_d = info_to_py(py, &r.info)?;
        Ok((obs_d, rew_d, term_d, trunc_d, info_d))
    }
}

/// Internal storage discriminating between buffer backends so we can keep
/// access to backend-specific methods (e.g. PER's [`PrioritizedReplayBuffer::beta`]).
enum ReplayInner {
    Uniform(UniformReplayBuffer),
    Prioritized(PrioritizedReplayBuffer),
}

impl ReplayInner {
    fn as_trait_mut(&mut self) -> &mut dyn ReplayBuffer {
        match self {
            ReplayInner::Uniform(b) => b,
            ReplayInner::Prioritized(b) => b,
        }
    }
    fn as_trait(&self) -> &dyn ReplayBuffer {
        match self {
            ReplayInner::Uniform(b) => b,
            ReplayInner::Prioritized(b) => b,
        }
    }
}

/// Numpy-friendly Replay buffer for RL / self-play training.
///
/// Two backends are exposed via the `kind` constructor argument:
/// - `"uniform"` uses [`UniformReplayBuffer`] (FIFO ring + uniform sampling).
/// - `"prioritized"` uses [`PrioritizedReplayBuffer`] (PER + SumTree).
///
/// `sample` returns a Python `dict` with numpy arrays so trainers can feed
/// the batch directly into a torch / jax model. Use `update_priorities` after
/// computing TD errors when running PER.
#[pyclass(name = "ReplayBuffer", module = "othello_sim")]
pub struct PyReplayBuffer {
    inner: ReplayInner,
    rng: ChaCha8Rng,
}

#[pymethods]
impl PyReplayBuffer {
    /// Create a replay buffer.
    ///
    /// - `capacity` : maximum number of transitions retained.
    /// - `kind` : `"uniform"` (default) or `"prioritized"`.
    /// - `alpha` / `beta` / `beta_increment` / `epsilon` : PER hyperparameters
    ///   (ignored for uniform).
    /// - `seed` : RNG seed for sampling.
    #[new]
    #[pyo3(signature = (
        capacity,
        kind = "uniform",
        alpha = 0.6,
        beta = 0.4,
        beta_increment = 0.001,
        epsilon = 1e-6,
        seed = None,
    ))]
    pub fn new(
        capacity: usize,
        kind: &str,
        alpha: f32,
        beta: f32,
        beta_increment: f32,
        epsilon: f32,
        seed: Option<u64>,
    ) -> PyResult<Self> {
        if capacity == 0 {
            return Err(PyValueError::new_err("capacity must be > 0"));
        }
        let lower = kind.to_ascii_lowercase();
        let inner = match lower.as_str() {
            "uniform" => ReplayInner::Uniform(UniformReplayBuffer::new(capacity)),
            "prioritized" | "per" => {
                let buf = PrioritizedReplayBuffer::new(capacity)
                    .with_alpha(alpha)
                    .with_beta(beta)
                    .with_beta_increment(beta_increment)
                    .with_epsilon(epsilon);
                ReplayInner::Prioritized(buf)
            }
            other => {
                return Err(PyValueError::new_err(format!(
                    "kind must be 'uniform' or 'prioritized', got {other:?}"
                )));
            }
        };
        let rng = ChaCha8Rng::seed_from_u64(seed.unwrap_or(0));
        Ok(Self { inner, rng })
    }

    /// Push one transition into the buffer.
    ///
    /// `observation` must be `(3, H, W)` float32, `legal_mask` must be `(H*W+1,)`
    /// bool, and `policy` (optional) must be `(H*W+1,)` float32.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        observation,
        action,
        value,
        legal_mask,
        side,
        move_number,
        game_id,
        policy = None,
    ))]
    pub fn push(
        &mut self,
        observation: PyReadonlyArray3<f32>,
        action: u32,
        value: f32,
        legal_mask: PyReadonlyArray1<bool>,
        side: &str,
        move_number: u32,
        game_id: &str,
        policy: Option<PyReadonlyArray1<f32>>,
    ) -> PyResult<()> {
        let side_color = parse_color(side)?;
        let obs_arr: Array3<f32> = observation.as_array().to_owned();
        let mask_arr: Array1<bool> = legal_mask.as_array().to_owned();
        let policy_arr: Option<Array1<f32>> = policy.map(|p| p.as_array().to_owned());
        let t = Transition {
            observation: obs_arr,
            action,
            policy: policy_arr,
            value,
            legal_mask: mask_arr,
            side: side_color,
            move_number,
            game_id: game_id.to_string(),
        };
        self.inner.as_trait_mut().push(t);
        Ok(())
    }

    /// Sample a batch and return it as a `dict` of numpy arrays.
    pub fn sample<'py>(
        &mut self,
        py: Python<'py>,
        batch_size: usize,
    ) -> PyResult<Bound<'py, PyDict>> {
        let batch = self
            .inner
            .as_trait_mut()
            .sample(batch_size, &mut self.rng)
            .map_err(|e| PyRuntimeError::new_err(format!("replay sample error: {e}")))?;
        let dict = PyDict::new_bound(py);
        dict.set_item("observations", batch.observations.into_pyarray_bound(py))?;
        dict.set_item("actions", batch.actions.into_pyarray_bound(py))?;
        dict.set_item("values", batch.values.into_pyarray_bound(py))?;
        dict.set_item("legal_masks", batch.legal_masks.into_pyarray_bound(py))?;
        match batch.policies {
            Some(p) => dict.set_item("policies", p.into_pyarray_bound(py))?,
            None => dict.set_item("policies", py.None())?,
        }
        let indices_arr: Array1<u64> = batch.indices.iter().map(|&i| i as u64).collect();
        dict.set_item("indices", indices_arr.into_pyarray_bound(py))?;
        match batch.weights {
            Some(w) => dict.set_item("weights", w.into_pyarray_bound(py))?,
            None => dict.set_item("weights", py.None())?,
        }
        Ok(dict)
    }

    /// Update PER priorities. `indices` and `priorities` must have equal length.
    /// No-op for uniform buffers.
    pub fn update_priorities(
        &mut self,
        indices: PyReadonlyArray1<u64>,
        priorities: PyReadonlyArray1<f32>,
    ) -> PyResult<()> {
        let idx_view = indices.as_array();
        let pri_view = priorities.as_array();
        let idx: Vec<usize> = idx_view.iter().map(|&v| v as usize).collect();
        let pri: Vec<f32> = pri_view.iter().copied().collect();
        self.inner
            .as_trait_mut()
            .update_priorities(&idx, &pri)
            .map_err(|e| PyRuntimeError::new_err(format!("update_priorities error: {e}")))?;
        Ok(())
    }

    /// Number of transitions currently stored.
    pub fn __len__(&self) -> usize {
        self.inner.as_trait().len()
    }

    /// Capacity of the buffer.
    #[getter]
    pub fn capacity(&self) -> usize {
        self.inner.as_trait().capacity()
    }

    /// True if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.as_trait().is_empty()
    }

    /// True if the buffer has reached `capacity`.
    pub fn is_full(&self) -> bool {
        self.inner.as_trait().is_full()
    }

    /// Drop all transitions.
    pub fn clear(&mut self) {
        self.inner.as_trait_mut().clear();
    }

    /// Returns `True` if this is a Prioritized Experience Replay buffer.
    #[getter]
    pub fn prioritized(&self) -> bool {
        matches!(self.inner, ReplayInner::Prioritized(_))
    }

    /// Backend kind string: `"uniform"` or `"prioritized"`.
    #[getter]
    pub fn kind(&self) -> &'static str {
        match self.inner {
            ReplayInner::Uniform(_) => "uniform",
            ReplayInner::Prioritized(_) => "prioritized",
        }
    }

    /// Current `beta` value used for IS-weight bias correction.
    /// Returns `None` for uniform buffers.
    pub fn beta(&self) -> Option<f32> {
        match &self.inner {
            ReplayInner::Uniform(_) => None,
            ReplayInner::Prioritized(b) => Some(b.beta()),
        }
    }
}

/// Module entry point.
#[pymodule]
fn othello_sim(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyOthelloEnv>()?;
    m.add_class::<PyOthelloMultiEnv>()?;
    m.add_class::<PyReplayBuffer>()?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    // pyo3 features = ["macros"] のみで extension-module を有効にしないとき，
    // PyO3 のリンカ要件は緩和される ( Python に動的リンクされない)．
    // ただし `Python::with_gil` を呼ぶと auto-initialize feature が必要なので，
    // ここでは Python ランタイムなしで動作するロジック ( `parse_*` 関数のみ) をテストする．

    use super::*;

    #[test]
    fn parse_color_works() {
        assert_eq!(parse_color("black").unwrap(), Color::Black);
        assert_eq!(parse_color("WHITE").unwrap(), Color::White);
        assert!(parse_color("foo").is_err());
    }

    #[test]
    fn parse_obs_type_works() {
        assert_eq!(parse_obs_type("planes").unwrap(), ObservationType::Planes);
        assert_eq!(
            parse_obs_type("move_sequence").unwrap(),
            ObservationType::MoveSequence
        );
        assert!(parse_obs_type("nope").is_err());
    }

    #[test]
    fn parse_reward_mode_works() {
        assert_eq!(parse_reward_mode("sparse").unwrap(), RewardMode::Sparse);
        assert_eq!(parse_reward_mode("DENSE").unwrap(), RewardMode::Dense);
        assert!(parse_reward_mode("nope").is_err());
    }
}
