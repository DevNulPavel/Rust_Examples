#![allow(unused_imports)]

mod ping_pong_test;

use actors_test;


fn main() {
    actors_test::test_actor_messages();
    ping_pong_test::test_ping_pong();
}