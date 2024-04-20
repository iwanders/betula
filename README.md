# Betula

![banner](./media/second_tick_tree.png)

This is a behaviour [tree](https://en.wikipedia.org/wiki/Birch) library. That comes with some 'batteries included'.
It is created with the goal of automating a computer game, but the library itself is agnostic to particular usecases.


# Crates

Brief overview of the crates in this workspace.

## betula_core
- Holds the traits for `Node` and `Tree`.
- The `basic` module holds the standard (non-event) implementation for a blackboard and a tree.
- Holds helpers for `Port`s and `BlackboardValue`.
### Nodes
  - `SuccessNode`: Always returns `Success`, may be a decorator.
  - `RunningNode`: Always returns `Running`, may be a decorator.
  - `FailureNode`: Always returns `Failure`, may be a decorator.
  - `SelectorNode`: Executes in order, returns first non-`Failure`.
  - `SequenceNode`: Executes in order, returns first non-`Success`.


## betula_common
Main components:
- `TreeSupport` to allow type-erased serialization and deserialization of the tree state (into/from `serde`).
- Control protocol to manipulate a tree.
- Server thread to allow running a tree in the background.

### Nodes
  - `TimeNode`: Write the unix time to a blackboard as `f64`.
  - `DelayNode`: Delays execution of the child node with the specified interval.

## betula_egui
- Uses the control protocol from `betula_common`.
- `UiNode` trait that must be implemented for `Node`s to provide editor support.
- `BetulaViewer`, backed by [egui-snarl](https://github.com/zakarumych/egui-snarl).
- `UiSupport` that allows registering new nodes.
- `Editor`, an `eframe::App` that can be instantiated.
- `UiNode` implementation for `betula_common` and `betula_core`.

## betula_demo
- Application that instantiates an editor with all nodes that exist in the workspace.

## betula_enigo
Betula node for [enigo](https://github.com/enigo-rs/enigo): `Cross platform input simulation in Rust`.
This crate provides both the `Node` as well as the `UiNode` if the `betula_egui` feature is enabled.

- Nodes:
  - `EnigoInstanceNode`: Provides an `Enigo` instance to the blackboard.
  - `EnigoNode`: Sends `Enigo::Token` to the `Enigo` instance to simulate events. 






# License
License is [`BSD-3-Clause`](./LICENSE).
