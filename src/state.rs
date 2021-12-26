use std::collections::HashMap;

use eframe::egui::CtxRef;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use snew::things::{Me, Post, PostFeed};

use crate::{
    components::{
        MainContentComponent, PostFeedComponent, PostId, PostSummaryComponent, ViewablePost,
    },
    config::Options,
    fetch::Fetcher,
    Render,
};

#[derive(Deserialize, Serialize)]
pub struct State {
    /// The post feed, a scrollable view of posts.
    pub feed_component: PostFeedComponent,
    /// The main, center view content.
    pub main_component: MainContentComponent,
    /// The summary of the the current post. Also information about the user, if present.
    pub summary_component: PostSummaryComponent,
    /// Currently loaded feed.
    #[serde(skip)]
    pub feed: Option<PostFeed>,
    /// Posts that are fetched and can be displayed
    #[serde(skip)]
    posts: Vec<ViewablePost>,
    #[serde(skip)]
    pub active_filters: HashMap<u32, fn(&&ViewablePost) -> bool>,
    /// Cached content
    #[serde(skip)]
    #[serde(default = "empty_map")]
    content_cache: LruCache<PostId, Option<Box<dyn Render>>>,
    /// Number of components claiming that keybinds should not be read.
    #[serde(skip)]
    pub num_request_disable_binds: u32,
    /// Reset posts
    #[serde(skip)]
    pub mark_for_refresh: bool,
    /// User options
    #[serde(skip)]
    pub options: Options,
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("feed_component", &self.feed_component)
            .field("main_component", &self.main_component)
            .field("summary_component", &self.summary_component)
            .field("feed", &self.feed)
            .field("posts", &self.posts)
            .field("content_cache", &self.content_cache)
            .field("num_request_disable_binds", &self.num_request_disable_binds)
            .field("mark_for_refresh", &self.mark_for_refresh)
            .field("options", &self.options)
            .finish()
    }
}

fn current_buffer<'a, T>(vec: &'a Vec<T>, idx: usize, amount: usize, ratio: f32) -> &'a [T] {
    let right_side = (ratio * amount as f32).round() as usize;
    let left_side = ((1f32 - ratio) * amount as f32).round() as usize;

    let len = vec.len();

    if len <= amount {
        &vec[..]
    } else if idx.checked_sub(left_side).is_none() {
        &vec[0..idx + right_side]
    } else if idx + right_side > len {
        &vec[idx - left_side..len - 1]
    } else {
        &vec[idx - left_side..idx + right_side]
    }
}

impl State {
    pub fn new(feed: PostFeed) -> Self {
        Self {
            feed_component: PostFeedComponent::new(),
            main_component: MainContentComponent::new(),
            summary_component: PostSummaryComponent::new(),
            feed: Some(feed),
            posts: vec![],
            active_filters: HashMap::new(),
            num_request_disable_binds: 0,
            mark_for_refresh: true,
            content_cache: LruCache::new(250),
            options: Default::default(),
        }
    }
    pub fn reset_feed(&mut self, new_feed: PostFeed) {
        self.feed = Some(new_feed);
        self.posts.clear();
        self.content_cache.clear();
        self.feed_component.reset();

        self.mark_for_refresh = true;
    }
    pub fn get_working_posts(&self) -> impl Iterator<Item = &ViewablePost> {
        Self::filter_posts(&self.posts, &self.active_filters)
    }

    pub fn unfiltered_len(&self) -> usize {
        self.posts.len()
    }

    pub fn buffer_posts(&mut self, fetcher: &mut Fetcher) {
        let current = Self::filter_posts(&self.posts, &self.active_filters).collect();
        let window = current_buffer(
            &current,
            self.feed_component.viewed,
            self.options.buffer_amount,
            self.options.buffer_ratio,
        );

        for post in window {
            if !self.content_cache.contains(&post.post_id) {
                self.content_cache.put(post.post_id, None);
                fetcher.get_content(post.inner.clone(), post.post_id)
            }
        }
    }

    pub fn set_feed(&mut self, mut feed: PostFeed) {
        assert!(self.feed.is_none());
        feed.limit = 15;
        self.feed = Some(feed);
    }

    pub fn extend_posts(&mut self, posts: Vec<Post>) {
        let mut idx = self.posts.len();

        for post in posts {
            self.posts.push((idx, post).into());
            idx += 1;
        }
    }

    pub fn set_content(&mut self, post_id: &PostId, content: Box<dyn Render>) {
        if let Some(empty_content) = self.content_cache.get_mut(post_id) {
            assert!(empty_content.is_none());
            *empty_content = Some(content);
        }
    }

    fn filter_posts<'a>(
        posts: &'a Vec<ViewablePost>,
        filters: &HashMap<u32, fn(&&ViewablePost) -> bool>,
    ) -> Box<dyn Iterator<Item = &'a ViewablePost> + 'a> {
        let mut iter: Box<dyn Iterator<Item = &ViewablePost>> = Box::new(posts.iter());
        for filter in filters.values() {
            iter = Box::new(iter.filter(filter.clone()));
        }

        iter
    }
}

impl State {
    pub fn render_summary_component(&self, ctx: &CtxRef, me: Option<&Me>) {
        self.summary_component.render(
            ctx,
            &self.options,
            self.posts.get(self.feed_component.viewed),
            me,
        );
    }

    pub fn render_feed_component(&mut self, ctx: &CtxRef, has_moved: bool) {
        self.feed_component.render(
            Self::filter_posts(&self.posts, &self.active_filters),
            ctx,
            &self.options,
            has_moved,
        );
    }

    pub fn render_main_content(&mut self, ctx: &CtxRef) {
        let post = Self::filter_posts(&self.posts, &self.active_filters)
            .skip(self.feed_component.viewed)
            .next();

        let content = Box::new(String::from("Loading..")) as Box<dyn Render>;

        let mut content = &content;

        if let Some(post) = post {
            if let Some(maybe_cached) = self.content_cache.get(&post.post_id) {
                if let Some(cached_content) = maybe_cached {
                    content = cached_content;
                }
            }
        }

        self.main_component.render(ctx, &self.options, content);
    }
}

fn empty_map() -> LruCache<PostId, Option<Box<dyn Render>>> {
    LruCache::new(250)
}
