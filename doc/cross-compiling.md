The rust compiler ran out of memory when I ran it directly on the Pi Zero.  One option is to compile on a different Pi (models other than the zero seem to have more ram). Another option is to compile it on a desktop/x86 computer and just copy the binary.  It's a little bit of a hassle, primarily because of an OpenSSL dependency.  However, it does work, and here are some rough notes on how I got it to go:

```
rustup target add arm-unknown-linux-gnueabihf
rustup target add armv7-unknown-linux-gnueabihf
```

Edit ~/.cargo/config to add:
```
[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"

[target.arm-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"
```

This helps us compile things for Raspberry Pi:
```
$ git clone https://github.com/raspberrypi/tools.git pitools
```

TTDash uses an HTTP library which depends on OpenSSL. So we have to download and compile OpenSSL for arm.  (Note that --cross-compile-prefix should point to the directory from the previous step.)
```
$ git clone https://github.com/openssl/openssl.git
$ cd openssl
$ git checkout --track origin/OpenSSL_1_1_1-stable
$ export INSTALL_DIR=~/arm/
$ ./Configure linux-generic32 shared --prefix=/home/mrjones/arm/ --openssldir=/home/mrjones/arm/openssl/ --cross-compile-prefix=~/src/pitools/arm-bcm2708/gcc-linaro-arm-linux-gnueabihf-raspbian-x64/bin/arm-linux-gnueabihf-
$ make depend && make && make install && make install
```

Now in the TTDash directory:
```
$ PATH=$PATH:/home/mrjones/src/pitools/arm-bcm2708/arm-linux-gnueabihf/bin/
$ export OPENSSL_INCLUDE_DIR=/home/mrjones/arm/include ; export OPENSSL_LIB_DIR=/home/mrjones/arm/lib ; cargo build --target arm-unknown-linux-gnueabihf
```

This was mostly inspired by:
[https://github.com/tiziano88/rust-raspberry-pi/blob/master/Dockerfile](https://github.com/tiziano88/rust-raspberry-pi/blob/master/Dockerfile)

For what it's worth [this StackOverflow answer](https://stackoverflow.com/questions/37375712/cross-compile-rust-openssl-for-raspberry-pi-2) also seemed promising, but I haven't tried it yet.
