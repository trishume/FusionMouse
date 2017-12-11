# FusionMouse

FusionMouse is currently an implementation of the [PolyMouse](https://github.com/trishume/PolyMouse) concept in Rust using a Tobii 4C eye tracker and a TrackIR 4 head tracker. It fuses head tracking and eye tracking to create a fast hands-free mouse replacement. I've tested it and I get approximately the same performance on Fitts' law pointing speed tests as I do with a mouse.

Unlike my previous PolyMouse implementation, this one is designed to be more than a research system and will hopefully eventually be ready for daily use. With the original PolyMouse it's practically impossible to gather all the dependencies to compile it, but this is a nice Rust crate that can compile and run after installing LinuxTrack and the Tobii SDK.

Currently it uses an alpha Tobii SDK for mac that you have to request to get, so it's not quite possible for others to use yet. But hopefully soon Tobii will release their SDK publicly.
