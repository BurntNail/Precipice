//! Bridge code for use of both synchronous and asynchronous receivers.
//!
//! Ideally, you use the [`ReceiverSynchronicity`] as a typestate, and keep a [`Receivers`] in the struct.
//! To then extract the reciever, use [`Sync::from_receivers`]

use self::sealed::Sealed;
use std::{mem::ManuallyDrop, sync::mpsc::Receiver};
use tokio::sync::mpsc::UnboundedReceiver as TokioReceiver;

mod sealed {
    //! Module for a trait ([`Sealed`] that cannot be implemented outside this file

    ///Sealed trait for typestate
    pub trait Sealed {}
}

///Trait for typestate for use in the bencher with some utility methods
pub trait ReceiverSynchronicity: Sealed {
    ///The reciever type this sync state uses
    type Receiver;
    ///Get a [`Receivers`] from the associated type
    fn from_receiver(f: Self::Receiver) -> Receivers;
    ///Get the associated type from a [`Receivers`]
    unsafe fn from_receivers(receivers: Receivers) -> Self::Receiver;
}

///Synchronous receiver using the standard library
pub struct SyncStdLib;
///Asynchronous unbounded reciever using [`tokio`]
pub struct AsyncTokio;

impl Sealed for SyncStdLib {}
impl Sealed for AsyncTokio {}

impl ReceiverSynchronicity for SyncStdLib {
    type Receiver = Receiver<()>;
    fn from_receiver(f: Self::Receiver) -> Receivers {
        Receivers {
            sync: ManuallyDrop::new(f),
        }
    }
    unsafe fn from_receivers(receivers: Receivers) -> Self::Receiver {
        ManuallyDrop::into_inner(receivers.sync)
    }
}
impl ReceiverSynchronicity for AsyncTokio {
    type Receiver = TokioReceiver<()>;
    fn from_receiver(f: Self::Receiver) -> Receivers {
        Receivers {
            not_sync: ManuallyDrop::new(f),
        }
    }
    unsafe fn from_receivers(receivers: Receivers) -> Self::Receiver {
        ManuallyDrop::into_inner(receivers.not_sync)
    }
}

///Union for a receiver - this is used for having one thing to store in both the async and sync versions
pub union Receivers {
    ///sync stdlib version
    sync: ManuallyDrop<Receiver<()>>,
    ///async [`tokio`] version. Called `not_sync` as `async` is a reserved keyword in Rust 2021.
    not_sync: ManuallyDrop<TokioReceiver<()>>,
}
