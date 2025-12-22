-- Crawl sessions table to track individual crawl runs
create table "crawl_sessions"(
  id integer not null primary key autoincrement,
  start_url text not null,
  started_at datetime not null default current_timestamp,
  completed_at datetime,
  status text not null default 'running' check (status in ('running', 'completed', 'failed')),
  pages_crawled integer not null default 0,
  errors_encountered integer not null default 0
);

-- Pages table to store crawled pages
create table "pages"(
  id integer not null primary key autoincrement,
  crawl_session_id integer not null,
  url text not null unique,
  title text,
  description text,
  content_hash text,
  status_code integer,
  content_type text,
  content_length integer,
  language text
,
  crawled_at datetime not null default current_timestamp,
  last_modified datetime,
  error_message text,
  foreign key (crawl_session_id) references crawl_sessions(id) on delete cascade
);

-- Links table to store links found on pages
create table "links"(
  id integer not null primary key autoincrement,
  source_page_id integer not null,
  target_url text not null,
  link_text text,
  link_type text default 'internal' check (link_type in ('internal', 'external')),
  discovered_at datetime not null default current_timestamp,
  foreign key (source_page_id) references pages(id) on delete cascade
);

-- Domains table for crawl configuration
create table "domains"(
  id integer not null primary key autoincrement,
  domain text not null unique,
  robots_txt text,
  crawl_delay real default 1.0,
  allow_crawl boolean not null default true,
  last_robots_check datetime
);

-- URL queue for URLs pending crawl
create table "url_queue"(
  id integer not null primary key autoincrement,
  crawl_session_id integer not null,
  url text not null,
  priority integer not null default 0,
  retry_count integer not null default 0,
  status text not null default 'pending' check (status in ('pending', 'processing', 'completed', 'failed')),
  queued_at datetime not null default current_timestamp,
  foreign key (crawl_session_id) references crawl_sessions(id) on delete cascade,
  unique (crawl_session_id, url)
);

-- Create indexes for better query performance
create index idx_pages_crawl_session on pages(crawl_session_id);

create index idx_pages_url on pages(url);

create index idx_links_source_page on links(source_page_id);

create index idx_url_queue_crawl_session on url_queue(crawl_session_id);

create index idx_url_queue_status on url_queue(status);

create index idx_domains_domain on domains(domain);

