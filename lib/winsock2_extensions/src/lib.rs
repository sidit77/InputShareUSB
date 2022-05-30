use std::convert::TryFrom;
use std::os::windows::io::AsRawSocket;
use std::io::{Error, Result};
use winapi::shared::windef::HWND;
use winapi::um::winsock2::{SOCKET, WSAAsyncSelect, WSAGetLastError, SOCKET_ERROR};

#[allow(non_upper_case_globals)]
mod flags {
    use bitflags::bitflags;
    use winapi::um::winsock2::{FD_READ, FD_WRITE, FD_OOB, FD_ACCEPT, FD_CONNECT, FD_CLOSE, FD_QOS,
                               FD_GROUP_QOS, FD_ROUTING_INTERFACE_CHANGE, FD_ADDRESS_LIST_CHANGE};

    bitflags! {
        pub struct NetworkEvents : winapi::ctypes::c_long {
            const Read = FD_READ;
            const Write = FD_WRITE;
            const OOB = FD_OOB;
            const Accept = FD_ACCEPT;
            const Connect = FD_CONNECT;
            const Close = FD_CLOSE;
            const QoS = FD_QOS;
            const GroupQoS = FD_GROUP_QOS;
            const RoutingInterfaceChange = FD_ROUTING_INTERFACE_CHANGE;
            const AddressListChange = FD_ADDRESS_LIST_CHANGE;
        }
    }
}

pub use flags::NetworkEvents;

pub trait WinSockExt : AsRawSocket {


    /// Modifies the socket to send `msg` to the event queue of `window` when one of the `events` occurs.
    /// This methods automatically sets the socket to non-blocking mode.
    /// This method internally uses [WSAAsyncSelect](https://docs.microsoft.com/en-us/windows/win32/api/winsock/nf-winsock-wsaasyncselect).
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn notify(&self, window: HWND, msg: u32, events: NetworkEvents) -> Result<()> {
        let socket = SOCKET::try_from(self.as_raw_socket()).unwrap();
        assert!(!window.is_null());
        unsafe {
            match WSAAsyncSelect(socket, window, msg, events.bits()) {
                0 => Ok(()),
                SOCKET_ERROR => Err(Error::from_raw_os_error(WSAGetLastError())),
                _ => panic!("Invalid return code")
            }
        }
    }
}

impl<T: AsRawSocket> WinSockExt for T {}