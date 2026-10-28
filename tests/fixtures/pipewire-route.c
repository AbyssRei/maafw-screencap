// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Owned test routing only. No product/session-manager policy. The private
// daemon has no session manager; route only exact upstream screenshot requests
// for the explicitly supplied mfs-p0-fixture node, never host/default sources.
#include <errno.h>
#include <inttypes.h>
#include <signal.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <pipewire/pipewire.h>

enum { MAX_ROUTES = 16 };
struct supervisor;
struct route {
    struct supervisor *owner;
    uint32_t id;
    bool used, validated, attempted;
    struct pw_node *node;
    struct pw_proxy *link;
    struct spa_hook node_listener, node_proxy_listener, link_listener;
};
struct supervisor {
    struct pw_main_loop *loop;
    struct pw_core *core;
    struct pw_registry *registry;
    struct spa_hook registry_listener, core_listener;
    struct route routes[MAX_ROUTES];
    uint32_t producer;
    char producer_text[16];
    const char *ready_file;
    int sync, shutdown_sync, error;
    bool producer_seen, stopping, ready_written;
    uint64_t created, destroyed, bound;
};
static void fail(struct supervisor *s, const char *operation, int code)
{
    if (!s->error) fprintf(stderr, "owned route failure: %s code=%d\n", operation, code);
    s->error = 1;
    pw_main_loop_quit(s->loop);
}
static void increment(struct supervisor *s, uint64_t *value)
{
    if (*value == UINT64_MAX) fail(s, "counter overflow", -EOVERFLOW);
    else ++*value;
}
static bool equal(const struct spa_dict *props, const char *key, const char *value)
{
    const char *actual = props ? spa_dict_lookup(props, key) : NULL;
    return actual && strcmp(actual, value) == 0;
}
static void close_link(struct route *route)
{
    if (!route->link) return;
    struct pw_proxy *proxy = route->link;
    route->link = NULL;
    spa_hook_remove(&route->link_listener);
    pw_proxy_destroy(proxy);
    increment(route->owner, &route->owner->destroyed);
}
static void close_route(struct route *route)
{
    if (!route->used) return;
    close_link(route);
    if (route->node) {
        struct pw_node *node = route->node;
        route->node = NULL;
        spa_hook_remove(&route->node_listener);
        spa_hook_remove(&route->node_proxy_listener);
        pw_proxy_destroy((struct pw_proxy*)node);
    }
    route->used = false;
}
static void link_removed(void *data) { close_link(data); }
static void link_destroyed(void *data)
{
    struct route *route = data;
    route->link = NULL;
    spa_hook_remove(&route->link_listener);
    increment(route->owner, &route->owner->destroyed);
}
static void link_bound(void *data, uint32_t id)
{
    struct route *route = data;
    increment(route->owner, &route->owner->bound);
    printf("owned route bound input=%u output=%u link=%u\n", route->id, route->owner->producer, id);
}
static void proxy_error(void *data, int seq, int code, const char *message)
{
    (void)seq;
    struct route *route = data;
    fprintf(stderr, "owned route proxy input=%u message=%s\n", route->id, message ? message : "");
    fail(route->owner, "proxy error", code);
}
static const struct pw_proxy_events link_events = {
    PW_VERSION_PROXY_EVENTS, .destroy = link_destroyed, .removed = link_removed,
    .bound = link_bound, .error = proxy_error,
};
static void connect_route(struct route *route)
{
    struct supervisor *s = route->owner;
    if (!s->producer_seen || !route->validated || route->attempted) return;
    route->attempted = true; // no automatic retries/reconnection of failed links
    char input[16];
    int length = snprintf(input, sizeof(input), "%u", route->id);
    if (length <= 0 || (size_t)length >= sizeof(input)) { fail(s, "input ID", -EOVERFLOW); return; }
    const struct spa_dict_item items[] = {
        { PW_KEY_LINK_OUTPUT_NODE, s->producer_text }, { PW_KEY_LINK_OUTPUT_PORT, "0" },
        { PW_KEY_LINK_INPUT_NODE, input }, { PW_KEY_LINK_INPUT_PORT, "0" },
        { PW_KEY_OBJECT_LINGER, "false" },
    };
    const struct spa_dict properties = SPA_DICT_INIT_ARRAY(items);
    route->link = pw_core_create_object(s->core, "link-factory", PW_TYPE_INTERFACE_Link,
                                       PW_VERSION_LINK, &properties, 0);
    if (!route->link) { fail(s, "link create", -errno); return; }
    increment(s, &s->created);
    pw_proxy_add_listener(route->link, &route->link_listener, &link_events, route);
    printf("owned route requested input=%u output=%u\n", route->id, s->producer);
}
static void node_info(void *data, const struct pw_node_info *info)
{
    struct route *route = data;
    const struct spa_dict *props = info ? info->props : NULL;
    if (!props || route->validated) return;
    // Read full node info: registry globals intentionally omit media.name and
    // node.target. The latter is the original numeric request encoded by native
    // pw_stream_connect, observed here without changing product properties.
    const char *name = spa_dict_lookup(props, PW_KEY_MEDIA_NAME);
    const char *target = spa_dict_lookup(props, "node.target");
    printf("owned route candidate input=%u media.name=%s node.target=%s\n",
           route->id, name ? name : "(missing)", target ? target : "(missing)");
    if (!equal(props, PW_KEY_MEDIA_NAME, "MaaFramework Screencap") ||
        !equal(props, "node.target", route->owner->producer_text)) return;
    if (!equal(props, PW_KEY_NODE_AUTOCONNECT, "true") ||
        spa_dict_lookup(props, PW_KEY_TARGET_OBJECT) != NULL) {
        fail(route->owner, "unexpected private autoconnect/target override", -EINVAL);
        return;
    }
    route->validated = true;
    connect_route(route);
}
static const struct pw_node_events node_events = { PW_VERSION_NODE_EVENTS, .info = node_info };
static void node_removed(void *data) { close_route(data); }
static void node_destroyed(void *data)
{
    struct route *route = data;
    route->node = NULL;
    spa_hook_remove(&route->node_listener);
    spa_hook_remove(&route->node_proxy_listener);
    close_route(route);
}
static const struct pw_proxy_events node_proxy_events = {
    PW_VERSION_PROXY_EVENTS, .destroy = node_destroyed, .removed = node_removed, .error = proxy_error,
};
static void registry_global(void *data, uint32_t id, uint32_t permissions,
                            const char *type, uint32_t version, const struct spa_dict *props)
{
    (void)permissions;
    struct supervisor *s = data;
    if (strcmp(type, PW_TYPE_INTERFACE_Node) != 0) return;
    if (id == s->producer) {
        if (!equal(props, PW_KEY_NODE_NAME, "mfs-p0-fixture") || !equal(props, PW_KEY_MEDIA_CLASS, "Video/Source")) {
            fail(s, "explicit producer identity", -EINVAL); return;
        }
        s->producer_seen = true;
        for (size_t i = 0; i < MAX_ROUTES; ++i) if (s->routes[i].used) connect_route(&s->routes[i]);
        return;
    }
    if (!equal(props, PW_KEY_MEDIA_CLASS, "Stream/Input/Video") ||
        !equal(props, PW_KEY_MEDIA_CATEGORY, "Capture") || !equal(props, PW_KEY_MEDIA_ROLE, "Screen")) return;
    struct route *route = NULL;
    for (size_t i = 0; i < MAX_ROUTES; ++i) {
        if (s->routes[i].used && s->routes[i].id == id) return;
        if (!s->routes[i].used && !route) route = &s->routes[i];
    }
    if (!route) { fail(s, "route capacity", -ENOSPC); return; }
    *route = (struct route){ .owner = s, .id = id, .used = true };
    route->node = pw_registry_bind(s->registry, id, PW_TYPE_INTERFACE_Node,
                                  version < PW_VERSION_NODE ? version : PW_VERSION_NODE, 0);
    if (!route->node) { fail(s, "bind candidate node", -errno); return; }
    pw_proxy_add_listener((struct pw_proxy*)route->node, &route->node_proxy_listener, &node_proxy_events, route);
    int code = pw_node_add_listener(route->node, &route->node_listener, &node_events, route);
    if (code < 0) fail(s, "node listener", code);
}
static void registry_remove(void *data, uint32_t id)
{
    struct supervisor *s = data;
    if (id == s->producer && !s->stopping) { fail(s, "owned producer removed", -EPIPE); return; }
    for (size_t i = 0; i < MAX_ROUTES; ++i) if (s->routes[i].used && s->routes[i].id == id) close_route(&s->routes[i]);
}
static const struct pw_registry_events registry_events = {
    PW_VERSION_REGISTRY_EVENTS, .global = registry_global, .global_remove = registry_remove,
};
static void core_done(void *data, uint32_t id, int seq)
{
    struct supervisor *s = data;
    if (id != PW_ID_CORE) return;
    if (s->stopping && seq == s->shutdown_sync) {
        // Drain the supervisor connection's earlier bound/removed events before
        // accounting and destroying remaining owned proxies, without a sleep.
        pw_main_loop_quit(s->loop);
        return;
    }
    if (seq != s->sync || s->ready_written) return;
    if (!s->producer_seen) { fail(s, "producer absent at registry sync", -ENOENT); return; }
    FILE *ready = fopen(s->ready_file, "wx");
    if (!ready) { fail(s, "ready file", -errno); return; }
    bool ok = fprintf(ready, "owned route supervisor ready producer=%u\n", s->producer) > 0;
    if (fclose(ready) != 0 || !ok) fail(s, "ready file write", -EIO);
    else s->ready_written = true;
}
static void core_error(void *data, uint32_t id, int seq, int code, const char *message)
{
    (void)id; (void)seq;
    fprintf(stderr, "owned route core message=%s\n", message ? message : "");
    fail(data, "core error", code);
}
static const struct pw_core_events core_events = {
    PW_VERSION_CORE_EVENTS, .done = core_done, .error = core_error,
};
static void stop(void *data, int number)
{
    (void)number;
    struct supervisor *s = data;
    if (s->stopping) return;
    s->stopping = true;
    s->shutdown_sync = pw_core_sync(s->core, PW_ID_CORE, 0);
    if (s->shutdown_sync < 0) fail(s, "shutdown sync", s->shutdown_sync);
}
int main(int argc, char **argv)
{
    if (argc != 3) return 2;
    char *end = NULL;
    errno = 0;
    unsigned long parsed = strtoul(argv[1], &end, 10);
    if (errno || !*argv[1] || !end || *end || parsed > UINT32_MAX) return 2;
    struct supervisor s = { .producer = (uint32_t)parsed, .ready_file = argv[2] };
    int length = snprintf(s.producer_text, sizeof(s.producer_text), "%u", s.producer);
    if (length <= 0 || (size_t)length >= sizeof(s.producer_text)) return 2;
    setvbuf(stdout, NULL, _IOLBF, 0);
    pw_init(NULL, NULL);
    s.loop = pw_main_loop_new(NULL);
    if (!s.loop) { pw_deinit(); return 1; }
    struct pw_context *context = pw_context_new(pw_main_loop_get_loop(s.loop), NULL, 0);
    if (!context) { s.error = 1; goto finish; }
    s.core = pw_context_connect(context, NULL, 0);
    if (!s.core) { s.error = 1; goto finish; }
    int code = pw_core_add_listener(s.core, &s.core_listener, &core_events, &s);
    if (code < 0) { s.error = 1; goto finish; }
    s.registry = pw_core_get_registry(s.core, PW_VERSION_REGISTRY, 0);
    if (!s.registry) { s.error = 1; goto finish; }
    code = pw_registry_add_listener(s.registry, &s.registry_listener, &registry_events, &s);
    if (code < 0) { s.error = 1; goto finish; }
    struct pw_loop *loop = pw_main_loop_get_loop(s.loop);
    if (!pw_loop_add_signal(loop, SIGTERM, stop, &s) || !pw_loop_add_signal(loop, SIGINT, stop, &s)) { s.error = 1; goto finish; }
    s.sync = pw_core_sync(s.core, PW_ID_CORE, 0);
    if (s.sync < 0 || pw_main_loop_run(s.loop) < 0) s.error = 1;
finish:
    if (s.registry) {
        spa_hook_remove(&s.registry_listener);
        pw_proxy_destroy((struct pw_proxy*)s.registry);
    }
    for (size_t i = 0; i < MAX_ROUTES; ++i) close_route(&s.routes[i]);
    if (s.core) {
        spa_hook_remove(&s.core_listener);
        if (pw_core_disconnect(s.core) < 0) s.error = 1;
    }
    if (context) pw_context_destroy(context);
    pw_main_loop_destroy(s.loop);
    pw_deinit();
    if (!s.stopping || s.created == 0 || s.created != s.destroyed || s.created != s.bound) s.error = 1;
    printf("owned route supervisor created=%" PRIu64 " destroyed=%" PRIu64 " bound=%" PRIu64 " error=%d\n", s.created, s.destroyed, s.bound, s.error);
    return s.error;
}
