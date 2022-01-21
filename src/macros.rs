// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

#[macro_export(local_inner_macros)]
macro_rules! get_session_mut {
    ($self:ident, $token:ident) => {
        if let Some(session) = $self.sessions.get_mut($token.0) {
            Ok(session)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "no such session",
            ))
        }
    };
}
