use crate::{
    components::core::wasm::message::{data, source_local, source_runtime},
    ecs::Entity,
    event,
    global::{on, CallbackReturn, EntityId, OnHandle},
};

#[cfg(any(feature = "client", feature = "server"))]
use crate::internal::{conversion::IntoBindgen, wit};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
/// Where a message came from.
pub enum Source {
    /// This message came from the runtime.
    Runtime,
    /// This message came from the corresponding serverside module.
    #[cfg(feature = "client")]
    Server,
    /// This message came from the corresponding clientside module and was sent from `user_id`.
    #[cfg(feature = "server")]
    Client {
        /// The user that sent this message.
        user_id: String,
    },
    /// This message came from another module on this side.
    Local(EntityId),
}
impl Source {
    /// Is this message from the runtime?
    pub fn runtime(self) -> bool {
        matches!(self, Source::Runtime)
    }

    #[cfg(feature = "server")]
    /// The user that sent this message, if any.
    pub fn client_user_id(self) -> Option<String> {
        if let Source::Client { user_id } = self {
            Some(user_id)
        } else {
            None
        }
    }

    #[cfg(feature = "server")]
    /// The entity ID of the player that sent this message, if any.
    pub fn client_entity_id(self) -> Option<EntityId> {
        let Some(user_id) = self.client_user_id() else { return None; };
        let Some(player_id) = crate::player::get_by_user_id(&user_id) else { return None; };
        Some(player_id)
    }

    fn from_entity(e: &Entity) -> Option<Self> {
        if e.has(source_runtime()) {
            return Some(Source::Runtime);
        }

        if let Some(module) = e.get(source_local()) {
            return Some(Source::Local(module));
        }

        #[cfg(feature = "client")]
        if e.has(crate::components::core::wasm::message::source_server()) {
            return Some(Source::Server);
        }

        #[cfg(feature = "server")]
        if let Some(user_id) =
            e.get(crate::components::core::wasm::message::source_client_user_id())
        {
            return Some(Source::Client { user_id });
        }

        None
    }

    /// The module on this side that sent this message, if any.
    pub fn local(self) -> Option<EntityId> {
        match self {
            Source::Local(id) => Some(id),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
/// The target for a originating message.
pub enum Target {
    /// A message to all other modules running on this side.
    LocalBroadcast,
    /// A message to a specific module running on this side.
    Local(EntityId),

    // Client
    /// An unreliable transmission to the server.
    ///
    /// Not guaranteed to be received, and must be below one kilobyte.
    #[cfg(feature = "client")]
    ServerUnreliable,
    /// A reliable transmission to the server (guaranteed to be received).
    #[cfg(feature = "client")]
    ServerReliable,

    // Server
    /// An unreliable transmission to all clients.
    ///
    /// Not guaranteed to be received, and must be below one kilobyte.
    #[cfg(feature = "server")]
    ClientBroadcastUnreliable,
    /// A reliable transmission to all clients (guaranteed to be received).
    #[cfg(feature = "server")]
    ClientBroadcastReliable,
    /// An unreliable transmission to a specific client.
    ///
    /// Not guaranteed to be received, and must be below one kilobyte.
    #[cfg(feature = "server")]
    ClientTargetedUnreliable(
        /// The user to send to.
        String,
    ),
    /// A reliable transmission to a specific client (guaranteed to be received).
    #[cfg(feature = "server")]
    ClientTargetedReliable(
        /// The user to send to.
        String,
    ),
}

#[cfg(feature = "client")]
impl IntoBindgen for Target {
    type Item = wit::client_message::Target;

    fn into_bindgen(self) -> Self::Item {
        match self {
            Target::ServerUnreliable => Self::Item::ServerUnreliable,
            Target::ServerReliable => Self::Item::ServerReliable,
            Target::LocalBroadcast => Self::Item::LocalBroadcast,
            Target::Local(id) => Self::Item::Local(id.into_bindgen()),
            #[cfg(feature = "server")]
            _ => unreachable!(),
        }
    }
}

#[cfg(feature = "server")]
impl<'a> IntoBindgen for &'a Target {
    type Item = wit::server_message::Target<'a>;

    fn into_bindgen(self) -> Self::Item {
        match self {
            Target::ClientBroadcastUnreliable => Self::Item::ClientBroadcastUnreliable,
            Target::ClientBroadcastReliable => Self::Item::ClientBroadcastReliable,
            Target::ClientTargetedUnreliable(user_id) => {
                Self::Item::ClientTargetedUnreliable(user_id.as_str())
            }
            Target::ClientTargetedReliable(user_id) => {
                Self::Item::ClientTargetedReliable(user_id.as_str())
            }
            Target::LocalBroadcast => Self::Item::LocalBroadcast,
            Target::Local(id) => Self::Item::Local(id.into_bindgen()),
            #[cfg(feature = "client")]
            _ => unreachable!(),
        }
    }
}

/// Send a message from this module to a specific `target`.
pub fn send<T: Message>(target: Target, data: &T) {
    #[cfg(all(feature = "client", not(feature = "server")))]
    wit::client_message::send(
        target.into_bindgen(),
        T::id(),
        &data.serialize_message().unwrap(),
    );
    #[cfg(all(feature = "server", not(feature = "client")))]
    wit::server_message::send(
        target.into_bindgen(),
        T::id(),
        &data.serialize_message().unwrap(),
    );
    #[cfg(any(
        all(not(feature = "server"), not(feature = "client")),
        all(feature = "server", feature = "client")
    ))]
    let _ = (target, data);
}

/// Subscribes to a message.
#[allow(clippy::collapsible_else_if)]
pub fn subscribe<R: CallbackReturn, T: Message>(
    callback: impl FnMut(Source, T) -> R + 'static,
) -> OnHandle {
    let mut callback = Box::new(callback);
    on(
        &format!("{}/{}", event::MODULE_MESSAGE, T::id()),
        move |e| {
            let source =
                Source::from_entity(e).context("No source available for incoming message")?;
            let data = e.get(data()).context("No data for incoming message")?;

            callback(source, T::deserialize_message(&data)?).into_result()?;
            Ok(())
        },
    )
}

/// Adds helpers for sending/subscribing to [Message]s.
pub trait MessageExt: Message {
    /// Sends this [Message] to `target`. Wrapper around [self::send].
    fn send(&self, target: Target) {
        self::send(target, self)
    }

    /// Subscribes to this [Message]. Wrapper around [self::subscribe].
    fn subscribe<R: CallbackReturn>(callback: impl FnMut(Source, Self) -> R + 'static) -> OnHandle {
        self::subscribe(callback)
    }
}
impl<T: Message> MessageExt for T {}

mod serde {
    pub use ambient_project_rt::message_serde::*;

    use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

    use crate::global::EntityId;

    impl MessageSerde for EntityId {
        fn serialize_message_part(&self, output: &mut Vec<u8>) -> Result<(), MessageSerdeError> {
            output.write_u64::<BigEndian>(self.id0)?;
            output.write_u64::<BigEndian>(self.id1)?;
            Ok(())
        }

        fn deserialize_message_part(
            input: &mut dyn std::io::Read,
        ) -> Result<Self, MessageSerdeError> {
            let (id0, id1) = (
                input.read_u64::<BigEndian>()?,
                input.read_u64::<BigEndian>()?,
            );
            Ok(Self { id0, id1 })
        }
    }
}
use anyhow::Context;
pub use serde::*;
