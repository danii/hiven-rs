hiven<span>.</span>rs
============
hiven<span>.</span>rs is an in-beta Rust library for automating user accounts on the beta chat application, [hiven.io](https://hiven.io/), and aims to provide complete coverage of their APIs.

It's easy to create a client and connect. Just call `Client::new` with your token, then invoke `start_gateway` on the returned client, with an event handler of your choosing.

> Notice: This library is in very early beta, expect many bugs and panics. That doesn't mean you shouldn't report them!

hiven<span>.</span>rs' Feature Set
----------------------------------
- [ ] Complete API coverage
	- [x] Message sending
	- [x] Message receiving
	- [ ] Message editing
	- [ ] Message deleting
	- [ ] Typing sending
	- [ ] Typing receiving
	- [ ] House building
	- [ ] House joining
	- [x] House (data) receiving
	- [ ] Room creating
	- [ ] Room deleting
	- [ ] Room editing
	- [x] Room (data) receiving
- [ ] Flexible
	- [ ] Custom addresses
	- [x] Event opt in
	- [ ] Builtin caching
- [x] Asynchronous
