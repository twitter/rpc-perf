//  Copyright 2019 Twitter, Inc
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

#[macro_use]
extern crate logger;

pub fn main() {
    println!("A simple demo of the logger");

    logger::Logger::new()
        .label("demo")
        .level(logger::Level::Trace)
        .init()
        .expect("Failed to initialize logger");
    trace!("Some tracing message");
    debug!("Some debugging message");
    info!("Just some general info");
    warn!("You might want to know this");
    error!("You need to know this");
    fatal!("Something really bad happened! Terminating program");
    // code below would be unreachable
}
