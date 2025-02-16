# airwire
## backstory
i have some speakers and a projector and it's too much effort to have a long cable to connect them. I also got a dac hat for one of my pis so I decided to roll my own audio transmitter/receiver.

## features
* goes over udp
* simple command line interface
* maybe supports multiple channels with `--channels` (have not tested not stereo stuff yet)
* configurable buffer size
* cross platform and light (tens of mb of memory usage, currently a little over 1mb release compile size)
* stereo swap in case your channel order differs for stereo.
* written in rust 
* optional opus stereo

## someday in the future
* configurable transports

## usage
### list devices for input and output
```bash
airwire enumerate
```
on linux this uses alsa, so it can't use pulseaudio/pipewire specific things.
### my typical dev setup
For Windows users: I highly recommend [this virtual loopback audio cable](https://vb-audio.com/Cable/), it makes an output also shows up as an audio input
```bash
cargo run --release -- transmit --target-device-name "CABLE Output (VB-Audio Virtual Cable)" --addr "192.168.68.96:6969" --stereo-swap
```
**`cargo run --release --` can be replaced with just the command `airwire` if you want to use this outside of development**
*todo: document linux monitor device usage, seems like this requires additional configuration*

then on a linux machine (in this case a pi, can leave buffer blank to automatically configure it):
```bash
RUST_BACKTRACE=full ./airwire recieve --addr "0.0.0.0:6969" --target-device-name pulse --buffer 480
```

## out of scope
* encryption
* automatically matching protocols and frame sizes with server (maybe in the future?)

## compilation
### linux
I compiled another copy of this project on a pi 5, and all I needed to do for dependencies at build time was `sudo apt install libasound2-dev`. By default opus support is enabled (btw I haven't tested without it), which requires `cmake`. So in addition you may need to run the following:
```bash
sudo apt install cmake
```
### windows
it worked out of the box on my machine, might have some stuff already installed.
### other platforms
it just needs the rust stdlib + cpal support, haven't tried


## special thanks
* claude for showing me a basic impl of a minimum viable product that this actually wasn't too hard if I used `cpal`. 
* `cpal` for being awesome for crossplatform low latency audio
* [@cutename](https://github.com/notcancername) from ssi for suggesting the extremely keep it simple solution of chucking pcm over the network.