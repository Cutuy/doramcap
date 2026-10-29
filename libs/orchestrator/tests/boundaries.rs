use orchestrator::{Error, Event, LocalExecutor, Result, State, Step};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    cell::{Cell, RefCell},
    marker::PhantomData,
    rc::Rc,
};

#[derive(Serialize, Deserialize)]
struct TrackedInput<T> {
    value: T,
    #[serde(skip)]
    original_dropped: Option<Rc<Cell<bool>>>,
}

impl<T> Drop for TrackedInput<T> {
    fn drop(&mut self) {
        if let Some(flag) = &self.original_dropped {
            flag.set(true);
        }
    }
}

struct InputLifetime {
    original_dropped: Rc<Cell<bool>>,
}

impl Step for InputLifetime {
    type Input = TrackedInput<String>;
    type Output = usize;

    async fn run(&mut self, input: Self::Input) -> Result<usize> {
        assert!(input.original_dropped.is_none());
        assert!(self.original_dropped.get());
        tokio::task::yield_now().await;
        assert!(self.original_dropped.get());
        Ok(input.value.len())
    }
}

#[tokio::test(flavor = "current_thread")]
async fn original_input_is_released_before_observation_factory_and_body() {
    for capture in [false, true] {
        // Arrange
        let dropped = Rc::new(Cell::new(false));
        let observed = Rc::new(RefCell::new(Vec::new()));
        let sink = observed.clone();
        let flag = dropped.clone();
        let mut executor = LocalExecutor::new(move |event: &Event| {
            sink.borrow_mut().push((event.state, flag.get()));
        })
        .capture_payloads(capture);
        let flag = dropped.clone();
        executor.register::<InputLifetime>(move || {
            assert!(
                flag.get(),
                "original input survived into factory construction"
            );
            InputLifetime {
                original_dropped: flag.clone(),
            }
        });
        let input = TrackedInput {
            value: "x".repeat(1024 * 1024),
            original_dropped: Some(dropped.clone()),
        };

        // Act
        let output = executor.run::<InputLifetime>(input).await.unwrap();

        // Assert
        assert_eq!(output.into_value(), 1024 * 1024);
        assert!(dropped.get());
        assert_eq!(
            *observed.borrow(),
            [(State::Started, true), (State::Succeeded, true)]
        );
        assert_eq!(executor.observer_failures(), 0);
    }
}

struct InvalidInput;

impl Step for InvalidInput {
    type Input = TrackedInput<f64>;
    type Output = ();

    async fn run(&mut self, _: Self::Input) -> Result<()> {
        panic!("invalid input entered body")
    }
}

#[tokio::test(flavor = "current_thread")]
async fn invalid_input_is_released_before_failure_observation() {
    // Arrange
    let dropped = Rc::new(Cell::new(false));
    let observed = Rc::new(RefCell::new(Vec::new()));
    let sink = observed.clone();
    let flag = dropped.clone();
    let mut executor = LocalExecutor::new(move |event: &Event| {
        sink.borrow_mut().push((event.state, flag.get()));
    });
    executor.register::<InvalidInput>(|| panic!("invalid input constructed step"));
    let input = TrackedInput {
        value: f64::NAN,
        original_dropped: Some(dropped.clone()),
    };

    // Act
    let result = executor.run::<InvalidInput>(input).await;

    // Assert
    assert!(matches!(
        result,
        Err(Error::Serialization {
            boundary: "input",
            ..
        })
    ));
    assert!(dropped.get());
    assert_eq!(
        *observed.borrow(),
        [(State::Started, true), (State::Failed, true)]
    );
    assert_eq!(executor.observer_failures(), 0);
}

#[derive(Default, Deserialize)]
struct PanicOnEncode;

impl Serialize for PanicOnEncode {
    fn serialize<S: serde::Serializer>(&self, _: S) -> std::result::Result<S::Ok, S::Error> {
        panic!("encode panic")
    }
}

#[derive(Default, Serialize)]
struct PanicOnDecode;

impl<'de> Deserialize<'de> for PanicOnDecode {
    fn deserialize<D: serde::Deserializer<'de>>(_: D) -> std::result::Result<Self, D::Error> {
        panic!("decode panic")
    }
}

#[derive(Serialize, Deserialize)]
struct PanicOnOriginalDrop {
    #[serde(skip)]
    original: bool,
}

impl Drop for PanicOnOriginalDrop {
    fn drop(&mut self) {
        assert!(!self.original, "original input destructor panic");
    }
}

struct CodecStep<I, O> {
    calls: Rc<Cell<u32>>,
    types: PhantomData<(I, O)>,
}

impl<I: Serialize + DeserializeOwned, O: Serialize + DeserializeOwned + Default> Step
    for CodecStep<I, O>
{
    type Input = I;
    type Output = O;

    async fn run(&mut self, _: I) -> Result<O> {
        self.calls.set(self.calls.get() + 1);
        Ok(O::default())
    }
}

fn assert_boundary_panic<I, O>(input: I, expected: &[State], expected_calls: u32)
where
    I: Serialize + DeserializeOwned + 'static,
    O: Serialize + DeserializeOwned + Default + 'static,
{
    // Arrange
    let observed = Rc::new(RefCell::new(Vec::new()));
    let sink = observed.clone();
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let mut executor = LocalExecutor::new(move |event: &Event| {
        sink.borrow_mut().push(event.state);
    });
    executor.register::<CodecStep<I, O>>(move || CodecStep {
        calls: count.clone(),
        types: PhantomData,
    });
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    // Act
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.block_on(executor.run::<CodecStep<I, O>>(input))
    }));

    // Assert
    assert!(result.is_err());
    assert_eq!(&*observed.borrow(), expected);
    assert_eq!(calls.get(), expected_calls);
    assert_eq!(executor.observer_failures(), 0);
}

#[test]
fn input_boundary_panics_emit_only_panicked() {
    assert_boundary_panic::<PanicOnEncode, ()>(PanicOnEncode, &[State::Panicked], 0);
    assert_boundary_panic::<PanicOnDecode, ()>(PanicOnDecode, &[State::Panicked], 0);
}

#[test]
fn original_input_destructor_panic_prevents_body_entry_and_emits_panicked() {
    assert_boundary_panic::<PanicOnOriginalDrop, ()>(
        PanicOnOriginalDrop { original: true },
        &[State::Panicked],
        0,
    );
}

#[test]
fn output_boundary_panics_follow_started_without_repeating_body() {
    let expected = &[State::Started, State::Panicked];
    assert_boundary_panic::<(), PanicOnEncode>((), expected, 1);
    assert_boundary_panic::<(), PanicOnDecode>((), expected, 1);
}
