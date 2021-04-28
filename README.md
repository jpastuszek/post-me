post-me
===============

Allows posting to PC in local network from phone that can read QR and has a browser.

Usage:
```sh
cargo install post-me
post-me
# Scan QR code for your PC local IP; fill the form and post.
# Posted data will be printed in the terminal.
```
TODO
----

* TLS
* Random free port
* Random URL prefix for endpoint obscurity
* Save posted files under file names in local directory
* `--serve` flag to also serve files from local directory: provide tree of all files, click to download, support for mime-types

Not planned:
* Firewall punching
* Relays


See also
--------

* [qrcp](https://github.com/claudiodangelis/qrcp) - basically what I wanted to bulid here but ready and in Go!
