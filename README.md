# rollback_client
an impl of a rollback client in rust using rust STD library networking components.

currently only connects to localhost and fakes network conditions, which is good enough for testing.

the plan eventually is to maybe release a crate?  i have no idea what sort of API i'd be looking at but once i get to full working rollback, ill hand out the repo to let other people give me criticisms

if i make a library things to consider: allow choice of serializer, allow choice of reader/writer for TCP, and send/recieve for UDP, maybe seperate out the actual network components from the rollbacking interface.

reference GGPO API for ideas
