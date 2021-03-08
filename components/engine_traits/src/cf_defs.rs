// Copyright 2019 TiKV Project Authors. Licensed under Apache-2.0.

pub type CfName = &'static str;

pub const CF_KEY_CHART: CfName = "key";
pub const CF_CLIENT_META: CfName = "cli_meta";
pub const CF_INDEX: CfName = "index";
pub const CF_INDEX_COUNTRY: CfName = "index_country";
pub const CF_INDEX_ROUTER: CfName = "index_router";
pub const CF_INDEX_PROVINCE: CfName = "index_province";
pub const CF_INDEX_MCDN: CfName = "index_mcdn";
pub const CF_INDEX_MCDN_COUNTRY: CfName = "index_mcdn_country";
pub const CF_INDEX_MCDN_CITY: CfName = "index_mcdn_city";
pub const CF_INDEX_MCDN_PROVINCE: CfName = "index_mcdn_province";

pub const CF_LIVE_CITY_INDEX: CfName = "live_city_index";
pub const CF_LIVE_COUNTRY_INDEX: CfName = "live_country_index";
pub const CF_LIVE_PROVINCE_INDEX: CfName = "live_province_index";

pub const CF_CONFIG: CfName = "version_config";
pub const CF_RESOURCE_META: CfName = "resource_meta";
pub const CF_STATS: CfName = "stats";
pub const CF_LOG: CfName = "log";
pub const CF_LIVE_REGISTER: CfName = "live_register";
pub const CF_LOGIN_TS: CfName = "login_ts";
pub const CF_GEOIP: CfName = "geoip";
pub const CF_CONNECTION_LIMIT: CfName = "connection_limit";
pub const CF_CONNECTION_LIMIT_TICKET: CfName = "connection_limit_ticket";
pub const CF_INDEX_NAT: CfName = "index_nat";

pub const CF_PUSH: CfName = "push_history";
pub const CF_PUSH_CID: CfName = "push_cid_history";
pub const CF_VISIT_HISTORY: CfName = "visit_history_cf";
pub const CF_RECENTLY_REPORT: CfName = "recently_report";
pub const CF_FIRST_VISIT: CfName = "first_visit";
pub const CF_VISIT_TIME_SERIES: CfName = "visit_time_series";
pub const CF_RESOURCE_VV: CfName = "resource_vv";
pub const CF_UPLOAD_TS: CfName = "upload_ts";
pub const CF_MVP: CfName = "mvp";
pub const CF_PREV_MVP: CfName = "prev_mvp";
pub const CF_RESOURCE_ID_MAP: CfName = "resource_map";
pub const CF_RECENTLY_WATCH: CfName = "recently_watch";
pub const CF_DEAD_CLIENT: CfName = "dead_client";
pub const CF_PUSH_QUOTA: CfName = "push_quota";

pub const CF_DEFAULT: CfName = "default";

pub const CF_RAFT: CfName = "raft";

pub const CF_LOCK: CfName = "lock";
pub const CF_WRITE: CfName = "write";
pub const CF_VER_DEFAULT: CfName = "ver_default";
// Cfs that should be very large generally.
pub const LARGE_CFS: &[CfName] = &[
    CF_DEFAULT,
    CF_CLIENT_META,
    CF_RESOURCE_META,
    CF_VISIT_HISTORY,
    CF_INDEX,
];
pub const ALL_CFS: &[CfName] = &[
    CF_DEFAULT,
    CF_KEY_CHART,
    CF_CLIENT_META,
    CF_INDEX,
    CF_PUSH,
    CF_RESOURCE_META,
    CF_STATS,
    CF_LOG,
    CF_VISIT_HISTORY,
    CF_RAFT,
];
pub const DATA_CFS: &[CfName] = &[
    CF_DEFAULT,
    CF_KEY_CHART,
    CF_CLIENT_META,
    CF_INDEX,
    CF_PUSH,
    CF_RESOURCE_META,
    CF_STATS,
    CF_LOG,
    CF_VISIT_HISTORY,
];

pub fn name_to_cf(name: &str) -> Option<CfName> {
    if name.is_empty() {
        return Some(CF_DEFAULT);
    }
    for c in ALL_CFS {
        if name == *c {
            return Some(c);
        }
    }

    None
}
