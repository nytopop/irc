//! Utilities and shortcuts for working with IRC servers.
//!
//! This module provides the [`ServerExt`](trait.ServerExt.html) trait which is the idiomatic way of
//! sending messages to an IRC server. This trait is automatically implemented for everything that
//! implements [`Server`](../trait.Server.html) and is designed to provide important functionality
//! without clutter.
//!
//! # Examples
//!
//! Using these APIs, we can connect to a server and send a one-off message (in this case,
//! identifying with the server).
//!
//! ```no_run
//! # extern crate irc;
//! use irc::client::prelude::{IrcServer, ServerExt};
//!
//! # fn main() {
//! let server = IrcServer::new("config.toml").unwrap();
//! // identify and send_privmsg both come from `ServerExt`
//! server.identify().unwrap();
//! server.send_privmsg("#example", "Hello, world!").unwrap();
//! # }
//! ```
//!
//! `ServerExt::identify` also plays an important role in performing IRCv3 capability negotiations.
//! In particular, calling `identify` will close the negotiations (and otherwise indicate IRCv3
//! compatibility). This means that all IRCv3 capability requests should be performed before calling
//! `identify`. For example:
//!
//! ```no_run
//! # extern crate irc;
//! # use irc::client::prelude::*;
//! # fn main() {
//! # let server = IrcServer::new("config.toml").unwrap(); 
//! server.send_cap_req(&[Capability::MultiPrefix, Capability::UserhostInNames]).unwrap();
//! server.identify().unwrap();
//! # }
//! ```
use std::borrow::ToOwned;

#[cfg(feature = "ctcp")]
use chrono::prelude::*;

use error::Result;
use proto::{Capability, Command, Mode, NegotiationVersion};
use proto::command::CapSubCommand::{END, LS, REQ};
use proto::command::Command::*;
use proto::mode::ModeType;
use client::server::Server;

/// Idiomatic extensions for sending messages to an IRC server.
pub trait ServerExt: Server {
    /// Sends a request for a list of server capabilities for a specific IRCv3 version.
    fn send_cap_ls(&self, version: NegotiationVersion) -> Result<()>
    where
        Self: Sized,
    {
        self.send(Command::CAP(
            None,
            LS,
            match version {
                NegotiationVersion::V301 => None,
                NegotiationVersion::V302 => Some("302".to_owned()),
            },
            None,
        ))
    }

    /// Sends an IRCv3 capabilities request for the specified extensions.
    fn send_cap_req(&self, extensions: &[Capability]) -> Result<()>
    where
        Self: Sized,
    {
        let append = |mut s: String, c| {
            s.push_str(c);
            s.push(' ');
            s
        };
        let mut exts = extensions.iter().map(|c| c.as_ref()).fold(
            String::new(),
            append,
        );
        let len = exts.len() - 1;
        exts.truncate(len);
        self.send(CAP(None, REQ, None, Some(exts)))
    }

    /// Sends a CAP END, NICK and USER to identify.
    fn identify(&self) -> Result<()>
    where
        Self: Sized,
    {
        // Send a CAP END to signify that we're IRCv3-compliant (and to end negotiations!).
        self.send(CAP(None, END, None, None))?;
        if self.config().password() != "" {
            self.send(PASS(self.config().password().to_owned()))?;
        }
        self.send(NICK(self.config().nickname()?.to_owned()))?;
        self.send(USER(
            self.config().username().to_owned(),
            "0".to_owned(),
            self.config().real_name().to_owned(),
        ))?;
        Ok(())
    }

    /// Sends a SASL AUTHENTICATE message with the specified data.
    fn send_sasl(&self, data: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.send(AUTHENTICATE(data.to_owned()))
    }

    /// Sends a SASL AUTHENTICATE request to use the PLAIN mechanism.
    fn send_sasl_plain(&self) -> Result<()>
    where
        Self: Sized,
    {
        self.send_sasl("PLAIN")
    }


    /// Sends a SASL AUTHENTICATE request to use the EXTERNAL mechanism.
    fn send_sasl_external(&self) -> Result<()>
    where
        Self: Sized,
    {
        self.send_sasl("EXTERNAL")
    }

    /// Sends a SASL AUTHENTICATE request to abort authentication.
    fn send_sasl_abort(&self) -> Result<()>
    where
        Self: Sized,
    {
        self.send_sasl("*")
    }

    /// Sends a PONG with the specified message.
    fn send_pong(&self, msg: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.send(PONG(msg.to_owned(), None))
    }

    /// Joins the specified channel or chanlist.
    fn send_join(&self, chanlist: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.send(JOIN(chanlist.to_owned(), None, None))
    }

    /// Joins the specified channel or chanlist using the specified key or keylist.
    fn send_join_with_keys(&self, chanlist: &str, keylist: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.send(JOIN(chanlist.to_owned(), Some(keylist.to_owned()), None))
    }

    /// Parts the specified channel or chanlist.
    fn send_part(&self, chanlist: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.send(PART(chanlist.to_owned(), None))
    }

    /// Attempts to oper up using the specified username and password.
    fn send_oper(&self, username: &str, password: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.send(OPER(username.to_owned(), password.to_owned()))
    }

    /// Sends a message to the specified target.
    fn send_privmsg(&self, target: &str, message: &str) -> Result<()>
    where
        Self: Sized,
    {
        for line in message.split("\r\n") {
            self.send(PRIVMSG(target.to_owned(), line.to_owned()))?
        }
        Ok(())
    }

    /// Sends a notice to the specified target.
    fn send_notice(&self, target: &str, message: &str) -> Result<()>
    where
        Self: Sized,
    {
        for line in message.split("\r\n") {
            self.send(NOTICE(target.to_owned(), line.to_owned()))?
        }
        Ok(())
    }

    /// Sets the topic of a channel or requests the current one.
    /// If `topic` is an empty string, it won't be included in the message.
    fn send_topic(&self, channel: &str, topic: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.send(TOPIC(
            channel.to_owned(),
            if topic.is_empty() {
                None
            } else {
                Some(topic.to_owned())
            },
        ))
    }

    /// Kills the target with the provided message.
    fn send_kill(&self, target: &str, message: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.send(KILL(target.to_owned(), message.to_owned()))
    }

    /// Kicks the listed nicknames from the listed channels with a comment.
    /// If `message` is an empty string, it won't be included in the message.
    fn send_kick(&self, chanlist: &str, nicklist: &str, message: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.send(KICK(
            chanlist.to_owned(),
            nicklist.to_owned(),
            if message.is_empty() {
                None
            } else {
                Some(message.to_owned())
            },
        ))
    }

    /// Changes the modes for the specified target.
    fn send_mode<T>(&self, target: &str, modes: &[Mode<T>]) -> Result<()>
    where
        Self: Sized,
        T: ModeType,
    {
        self.send(T::mode(target, modes))
    }

    /// Changes the mode of the target by force.
    /// If `modeparams` is an empty string, it won't be included in the message.
    fn send_samode(&self, target: &str, mode: &str, modeparams: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.send(SAMODE(
            target.to_owned(),
            mode.to_owned(),
            if modeparams.is_empty() {
                None
            } else {
                Some(modeparams.to_owned())
            },
        ))
    }

    /// Forces a user to change from the old nickname to the new nickname.
    fn send_sanick(&self, old_nick: &str, new_nick: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.send(SANICK(old_nick.to_owned(), new_nick.to_owned()))
    }

    /// Invites a user to the specified channel.
    fn send_invite(&self, nick: &str, chan: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.send(INVITE(nick.to_owned(), chan.to_owned()))
    }

    /// Quits the server entirely with a message.
    /// This defaults to `Powered by Rust.` if none is specified.
    fn send_quit(&self, msg: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.send(QUIT(Some(if msg.is_empty() {
            "Powered by Rust.".to_owned()
        } else {
            msg.to_owned()
        })))
    }

    /// Sends a CTCP-escaped message to the specified target.
    /// This requires the CTCP feature to be enabled.
    #[cfg(feature = "ctcp")]
    fn send_ctcp(&self, target: &str, msg: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.send_privmsg(target, &format!("\u{001}{}\u{001}", msg)[..])
    }

    /// Sends an action command to the specified target.
    /// This requires the CTCP feature to be enabled.
    #[cfg(feature = "ctcp")]
    fn send_action(&self, target: &str, msg: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.send_ctcp(target, &format!("ACTION {}", msg)[..])
    }

    /// Sends a finger request to the specified target.
    /// This requires the CTCP feature to be enabled.
    #[cfg(feature = "ctcp")]
    fn send_finger(&self, target: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.send_ctcp(target, "FINGER")
    }

    /// Sends a version request to the specified target.
    /// This requires the CTCP feature to be enabled.
    #[cfg(feature = "ctcp")]
    fn send_version(&self, target: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.send_ctcp(target, "VERSION")
    }

    /// Sends a source request to the specified target.
    /// This requires the CTCP feature to be enabled.
    #[cfg(feature = "ctcp")]
    fn send_source(&self, target: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.send_ctcp(target, "SOURCE")
    }

    /// Sends a user info request to the specified target.
    /// This requires the CTCP feature to be enabled.
    #[cfg(feature = "ctcp")]
    fn send_user_info(&self, target: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.send_ctcp(target, "USERINFO")
    }

    /// Sends a finger request to the specified target.
    /// This requires the CTCP feature to be enabled.
    #[cfg(feature = "ctcp")]
    fn send_ctcp_ping(&self, target: &str) -> Result<()>
    where
        Self: Sized,
    {
        let time = Local::now();
        self.send_ctcp(target, &format!("PING {}", time.timestamp())[..])
    }

    /// Sends a time request to the specified target.
    /// This requires the CTCP feature to be enabled.
    #[cfg(feature = "ctcp")]
    fn send_time(&self, target: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.send_ctcp(target, "TIME")
    }
}

impl<S> ServerExt for S
where
    S: Server,
{
}

#[cfg(test)]
mod test {
    use super::ServerExt;
    use client::data::Config;
    use client::server::IrcServer;
    use client::server::test::{get_server_value, test_config};
    use proto::{ChannelMode, Mode};

    #[test]
    fn identify() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.identify().unwrap();
        assert_eq!(
            &get_server_value(server)[..],
            "CAP END\r\nNICK :test\r\n\
             USER test 0 * :test\r\n"
        );
    }

    #[test]
    fn identify_with_password() {
        let server = IrcServer::from_config(Config {
            nickname: Some(format!("test")),
            password: Some(format!("password")),
            ..test_config()
        }).unwrap();
        server.identify().unwrap();
        assert_eq!(
            &get_server_value(server)[..],
            "CAP END\r\nPASS :password\r\nNICK :test\r\n\
             USER test 0 * :test\r\n"
        );
    }

    #[test]
    fn send_pong() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_pong("irc.test.net").unwrap();
        assert_eq!(&get_server_value(server)[..], "PONG :irc.test.net\r\n");
    }

    #[test]
    fn send_join() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_join("#test,#test2,#test3").unwrap();
        assert_eq!(
            &get_server_value(server)[..],
            "JOIN #test,#test2,#test3\r\n"
        );
    }

    #[test]
    fn send_part() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_part("#test").unwrap();
        assert_eq!(&get_server_value(server)[..], "PART #test\r\n");
    }

    #[test]
    fn send_oper() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_oper("test", "test").unwrap();
        assert_eq!(&get_server_value(server)[..], "OPER test :test\r\n");
    }

    #[test]
    fn send_privmsg() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_privmsg("#test", "Hi, everybody!").unwrap();
        assert_eq!(
            &get_server_value(server)[..],
            "PRIVMSG #test :Hi, everybody!\r\n"
        );
    }

    #[test]
    fn send_notice() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_notice("#test", "Hi, everybody!").unwrap();
        assert_eq!(
            &get_server_value(server)[..],
            "NOTICE #test :Hi, everybody!\r\n"
        );
    }

    #[test]
    fn send_topic_no_topic() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_topic("#test", "").unwrap();
        assert_eq!(&get_server_value(server)[..], "TOPIC #test\r\n");
    }

    #[test]
    fn send_topic() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_topic("#test", "Testing stuff.").unwrap();
        assert_eq!(
            &get_server_value(server)[..],
            "TOPIC #test :Testing stuff.\r\n"
        );
    }

    #[test]
    fn send_kill() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_kill("test", "Testing kills.").unwrap();
        assert_eq!(
            &get_server_value(server)[..],
            "KILL test :Testing kills.\r\n"
        );
    }

    #[test]
    fn send_kick_no_message() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_kick("#test", "test", "").unwrap();
        assert_eq!(&get_server_value(server)[..], "KICK #test test\r\n");
    }

    #[test]
    fn send_kick() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_kick("#test", "test", "Testing kicks.").unwrap();
        assert_eq!(
            &get_server_value(server)[..],
            "KICK #test test :Testing kicks.\r\n"
        );
    }

    #[test]
    fn send_mode_no_modeparams() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_mode("#test", &[Mode::Plus(ChannelMode::InviteOnly, None)]).unwrap();
        assert_eq!(&get_server_value(server)[..], "MODE #test +i\r\n");
    }

    #[test]
    fn send_mode() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_mode("#test", &[Mode::Plus(ChannelMode::Oper, Some("test".to_owned()))])
              .unwrap();
        assert_eq!(&get_server_value(server)[..], "MODE #test +o test\r\n");
    }

    #[test]
    fn send_samode_no_modeparams() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_samode("#test", "+i", "").unwrap();
        assert_eq!(&get_server_value(server)[..], "SAMODE #test +i\r\n");
    }

    #[test]
    fn send_samode() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_samode("#test", "+o", "test").unwrap();
        assert_eq!(&get_server_value(server)[..], "SAMODE #test +o test\r\n");
    }

    #[test]
    fn send_sanick() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_sanick("test", "test2").unwrap();
        assert_eq!(&get_server_value(server)[..], "SANICK test test2\r\n");
    }

    #[test]
    fn send_invite() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_invite("test", "#test").unwrap();
        assert_eq!(&get_server_value(server)[..], "INVITE test #test\r\n");
    }

    #[test]
    #[cfg(feature = "ctcp")]
    fn send_ctcp() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_ctcp("test", "MESSAGE").unwrap();
        assert_eq!(
            &get_server_value(server)[..],
            "PRIVMSG test :\u{001}MESSAGE\u{001}\r\n"
        );
    }

    #[test]
    #[cfg(feature = "ctcp")]
    fn send_action() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_action("test", "tests.").unwrap();
        assert_eq!(
            &get_server_value(server)[..],
            "PRIVMSG test :\u{001}ACTION tests.\u{001}\r\n"
        );
    }

    #[test]
    #[cfg(feature = "ctcp")]
    fn send_finger() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_finger("test").unwrap();
        assert_eq!(
            &get_server_value(server)[..],
            "PRIVMSG test :\u{001}FINGER\u{001}\r\n"
        );
    }

    #[test]
    #[cfg(feature = "ctcp")]
    fn send_version() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_version("test").unwrap();
        assert_eq!(
            &get_server_value(server)[..],
            "PRIVMSG test :\u{001}VERSION\u{001}\r\n"
        );
    }

    #[test]
    #[cfg(feature = "ctcp")]
    fn send_source() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_source("test").unwrap();
        assert_eq!(
            &get_server_value(server)[..],
            "PRIVMSG test :\u{001}SOURCE\u{001}\r\n"
        );
    }

    #[test]
    #[cfg(feature = "ctcp")]
    fn send_user_info() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_user_info("test").unwrap();
        assert_eq!(
            &get_server_value(server)[..],
            "PRIVMSG test :\u{001}USERINFO\u{001}\r\n"
        );
    }

    #[test]
    #[cfg(feature = "ctcp")]
    fn send_ctcp_ping() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_ctcp_ping("test").unwrap();
        let val = get_server_value(server);
        println!("{}", val);
        assert!(val.starts_with("PRIVMSG test :\u{001}PING "));
        assert!(val.ends_with("\u{001}\r\n"));
    }

    #[test]
    #[cfg(feature = "ctcp")]
    fn send_time() {
        let server = IrcServer::from_config(test_config()).unwrap();
        server.send_time("test").unwrap();
        assert_eq!(
            &get_server_value(server)[..],
            "PRIVMSG test :\u{001}TIME\u{001}\r\n"
        );
    }
}
