use orchestrator::{Error, Event, Input, LocalExecutor, Result, State, Step};
use serde::{Deserialize, Serialize};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::Duration,
};

struct Add {
    increment: Rc<Cell<u32>>,
    calls: u32,
}
impl Step for Add {
    type Input = u32;
    type Output = u32;
    async fn run(&mut self, input: u32) -> Result<u32> {
        self.calls += 1;
        let local = Rc::new(input);
        tokio::task::yield_now().await;
        Ok(*local + self.increment.get() + self.calls)
    }
}
fn executor(increment: u32) -> (LocalExecutor, Rc<RefCell<Vec<Event>>>) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    let mut executor = LocalExecutor::new(move |e: &Event| sink.borrow_mut().push(e.clone()));
    let increment = Rc::new(Cell::new(increment));
    executor.register::<Add>(move || Add {
        increment: increment.clone(),
        calls: 0,
    });
    executor.register::<Pending>(|| Pending);
    executor.register::<Panics>(|| Panics);
    (executor, events)
}
#[tokio::test(flavor = "current_thread")]
async fn fresh_instances_and_explicit_dependencies() {
    // Arrange
    let (executor, events) = executor(10);
    // Act
    let a = executor.run::<Add>(1).await.unwrap();
    let b = executor.run::<Add>(a.clone()).await.unwrap();
    let c = executor
        .run::<Add>(Input::new(0).after(&a).after(&a).after(&b))
        .await
        .unwrap();
    // Assert
    assert_eq!(*a.value(), 12);
    assert_eq!(*b.value(), 23);
    assert_eq!(*c.value(), 11);
    assert_eq!(b.invocation().dependencies, vec![a.invocation().id]);
    assert_eq!(
        c.invocation().dependencies,
        vec![a.invocation().id, b.invocation().id]
    );
    assert_eq!(events.borrow().len(), 6);
    assert!(events.borrow().iter().all(|e| e.payload.is_none()));
    // Act
    let (other, _) = self::executor(10);
    let result = other.run::<Add>(a).await;
    // Assert
    assert!(matches!(result, Err(Error::InvalidDependency)));
}
struct Parent {
    executor: Weak<LocalExecutor>,
}
impl Step for Parent {
    type Input = u32;
    type Output = u32;
    async fn run(&mut self, input: u32) -> Result<u32> {
        Ok(self
            .executor
            .upgrade()
            .unwrap()
            .run::<Add>(input)
            .await?
            .into_value())
    }
}
#[tokio::test(flavor = "current_thread")]
async fn nesting_and_concurrent_coroutines() {
    // Arrange
    let (mut executor, events) = executor(1);
    let executor = Rc::new_cyclic(|weak: &Weak<LocalExecutor>| {
        let weak = weak.clone();
        executor.register::<Parent>(move || Parent {
            executor: weak.clone(),
        });
        executor
    });
    // Act
    let (a, b) = tokio::join!(executor.run::<Parent>(2), executor.run::<Add>(3));
    // Assert
    assert_eq!(*a.unwrap().value(), 4);
    let sibling = b.unwrap();
    assert_eq!(*sibling.value(), 5);
    assert_eq!(sibling.invocation().parent, None);
    let events = events.borrow();
    let parent = events
        .iter()
        .find(|e| e.invocation.step.ends_with("Parent"))
        .unwrap();
    assert!(
        events
            .iter()
            .any(|e| e.invocation.parent == Some(parent.invocation.id))
    );
}
struct Pending;
impl Step for Pending {
    type Input = ();
    type Output = ();
    async fn run(&mut self, _: ()) -> Result<()> {
        std::future::pending().await
    }
}
#[tokio::test(flavor = "current_thread")]
async fn cancellation_emits_one_terminal_event() {
    // Arrange
    let (executor, events) = executor(1);
    // Act
    assert!(
        tokio::time::timeout(Duration::from_millis(5), executor.run::<Pending>(()))
            .await
            .is_err()
    );
    // Assert
    assert_eq!(
        events.borrow().iter().map(|e| e.state).collect::<Vec<_>>(),
        vec![State::Started, State::Cancelled]
    );
}
struct Echo;
impl Step for Echo {
    type Input = String;
    type Output = String;
    async fn run(&mut self, input: String) -> Result<String> {
        Ok(input)
    }
}
#[tokio::test(flavor = "current_thread")]
async fn payload_capture_and_large_local_values() {
    // Arrange
    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    let mut executor = LocalExecutor::new(move |e: &Event| sink.borrow_mut().push(e.clone()))
        .capture_payloads(true);
    executor.register::<Echo>(|| Echo);
    let value = "a".repeat(1024 * 1024 + 1);
    // Act
    let result = executor.run::<Echo>(value.clone()).await.unwrap();
    // Assert
    assert_eq!(result.value(), &value);
    assert_eq!(events.borrow()[0].payload, Some(serde_json::json!(value)));
    assert_eq!(events.borrow()[1].state, State::Succeeded);
}
#[tokio::test(flavor = "current_thread")]
async fn observer_failure_does_not_repeat_side_effects() {
    // Arrange
    let mut executor = LocalExecutor::new(|_: &Event| panic!("observer failed"));
    executor.register::<Add>(|| Add {
        increment: Rc::new(Cell::new(0)),
        calls: 0,
    });
    // Act
    let value = executor.run::<Add>(1).await.unwrap();
    // Assert
    assert_eq!(*value.value(), 2);
    assert_eq!(executor.observer_failures(), 2);
}
struct InvalidOutput;
impl Step for InvalidOutput {
    type Input = ();
    type Output = f64;
    async fn run(&mut self, _: ()) -> Result<f64> {
        Ok(f64::NAN)
    }
}
#[tokio::test(flavor = "current_thread")]
async fn output_must_deserialize_before_success() {
    // Arrange
    let mut executor = LocalExecutor::default();
    executor.register::<InvalidOutput>(|| InvalidOutput);
    // Act
    let result = executor.run::<InvalidOutput>(()).await;
    // Assert
    assert!(matches!(
        result,
        Err(Error::Serialization {
            boundary: "output",
            ..
        })
    ));
}
struct Panics;
impl Step for Panics {
    type Input = ();
    type Output = ();
    async fn run(&mut self, _: ()) -> Result<()> {
        panic!("step panic")
    }
}
#[test]
fn panic_unwinds_with_terminal_event() {
    // Arrange
    let (executor, events) = executor(0);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    // Act
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.block_on(executor.run::<Panics>(()))
    }));
    // Assert
    assert!(result.is_err());
    assert_eq!(events.borrow().last().unwrap().state, State::Panicked);
    assert_eq!(events.borrow().len(), 2);
}

// These deliberately normalize on decode, proving both local boundaries decode.
#[derive(Serialize, Deserialize)]
struct Portable {
    value: u32,
    #[serde(skip)]
    transient: u32,
}
struct Normalize;
impl Step for Normalize {
    type Input = Portable;
    type Output = Portable;
    async fn run(&mut self, input: Portable) -> Result<Portable> {
        assert_eq!(input.transient, 0);
        Ok(Portable {
            value: input.value,
            transient: 99,
        })
    }
}
#[tokio::test(flavor = "current_thread")]
async fn both_boundaries_really_decode_locally() {
    // Arrange
    let mut executor = LocalExecutor::default();
    executor.register::<Normalize>(|| Normalize);
    // Act
    let result = executor
        .run::<Normalize>(Portable {
            value: 7,
            transient: 42,
        })
        .await
        .unwrap();
    // Assert
    assert_eq!(result.value().value, 7);
    assert_eq!(result.value().transient, 0);
}
struct InvalidInput;
impl Step for InvalidInput {
    type Input = f64;
    type Output = ();
    async fn run(&mut self, _: f64) -> Result<()> {
        panic!("invalid input entered body")
    }
}
#[tokio::test(flavor = "current_thread")]
async fn invalid_input_never_constructs_step_and_missing_factory_is_explicit() {
    // Arrange
    let (mut executor, events) = executor(0);
    executor.register::<InvalidInput>(|| panic!("invalid input constructed step"));
    // Act
    let invalid = executor.run::<InvalidInput>(f64::NAN).await;
    let absent = executor.run::<Echo>(String::new()).await;
    // Assert
    assert!(matches!(
        invalid,
        Err(Error::Serialization {
            boundary: "input",
            ..
        })
    ));
    assert!(matches!(absent, Err(Error::UnregisteredStep(_))));
    assert_eq!(
        events.borrow().iter().map(|e| e.state).collect::<Vec<_>>(),
        vec![State::Started, State::Failed]
    );
}

struct Counter {
    count: Cell<u32>,
}
struct Count {
    singleton: Rc<Counter>,
    events: Rc<RefCell<Vec<Event>>>,
}
impl Step for Count {
    type Input = ();
    type Output = u32;
    async fn run(&mut self, _: ()) -> Result<u32> {
        // The same injected object is visible to both the step and observer.
        assert!(!self.events.borrow().is_empty());
        self.singleton.count.set(self.singleton.count.get() + 1);
        Ok(self.singleton.count.get())
    }
}
#[tokio::test(flavor = "current_thread")]
async fn shared_services_survive_steps_and_release_with_factory() {
    // Arrange
    let singleton = Rc::new(Counter {
        count: Cell::new(0),
    });
    let weak = Rc::downgrade(&singleton);
    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    let mut executor = LocalExecutor::new(move |e: &Event| sink.borrow_mut().push(e.clone()));
    executor.register::<Count>(move || Count {
        singleton: singleton.clone(),
        events: events.clone(),
    });
    // Act
    let a = executor.run::<Count>(()).await.unwrap();
    let b = executor.run::<Count>(()).await.unwrap();
    // Assert
    assert_eq!((*a.value(), *b.value()), (1, 2));
    assert_eq!(Rc::strong_count(&weak.upgrade().unwrap()), 2); // factory + temporary
    // Act
    drop(executor);
    // Assert
    assert!(weak.upgrade().is_none()); // Outputs retain provenance, not services.
}
