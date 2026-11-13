//! Native Agent transport. Authentication is connection-scoped, never task data.
mod security;
mod stream;
pub use security::{ApiToken, ClientSecurity, ServerSecurity};
pub use stream::{Connection, SocketIo};

#[cfg(test)]
mod latency_tests;
#[cfg(test)]
mod tests;
