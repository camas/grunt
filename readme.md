# Grunt

World of Warcraft Addon Manager+

With "+" obviously meaning "and stuff"

## Should I use this?

Probably not. It's mostly made with my setup in mind and because I wanted to make something useful in rust.

It does have a few interesting parts though

## Interesting Parts

* Automatic resolving of addons without having to reinstall them like most other cli addon managers. Mostly possible due to a working implementation of Twitch/Curse fingerprinting and api.

* Update TradeSkillMaster data. Reversed the private api their app uses.

* Faster updating by using multithreading + rust.
