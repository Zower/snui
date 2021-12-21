use std::{sync::Arc, thread};

use snew::things::{Post, PostFeed};

use crossbeam_channel::Sender;

pub enum Message {
    PostsReady(Vec<Arc<Post>>, PostFeed),
}

pub fn spawn_more(mut feed: PostFeed, s: Sender<Message>) {
    thread::spawn(move || {
        let posts: Vec<Arc<Post>> = feed
            .by_ref()
            .filter_map(|p| p.ok())
            .map(|p| Arc::new(p))
            .take(35)
            .collect();

        let _x = s.send(Message::PostsReady(posts, feed));
    });
}
