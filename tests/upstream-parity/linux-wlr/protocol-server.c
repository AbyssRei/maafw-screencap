// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Owned native IPC/SHM protocol fixture, not a compositor or desktop capture.
#include <wayland-server-core.h>
#include <wayland-server-protocol.h>
#include "screencopy-server.h"
#include "pointer-server.h"
#include "keyboard-server.h"
#include "protocol-pattern.h"
#include <signal.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <dlfcn.h>
#include <limits.h>

struct server;
struct registry_output {
    struct server* server;
    struct wl_global* global;
    unsigned generation;
};
struct server {
    struct wl_display* display;
    const char* directory;
    unsigned version, managers_created, managers_destroyed, frames_created, frames_destroyed, copies;
    int error;
    bool stopped;
    bool registry;
    bool failures;
    struct registry_output outputs[22];
    unsigned output_globals, output_removed, output_binds, output_destroys;
    FILE* registry_rows;
    FILE* failure_rows;
    unsigned failed_frames, disconnected_clients;
    uint32_t failure_buffers[20];
};
struct manager { struct server* server; unsigned next, cycle; };
struct frame { struct server* server; struct mfs_protocol_case input; bool copied; unsigned cycle, phase; };
static bool registry_advance(struct server* s, unsigned phase, unsigned round);
static void failure(struct server* s, const char* message)
{
    fprintf(stderr,"protocol fixture failure: %s\n",message);
    s->error = 1;
    wl_display_terminate(s->display);
}
static void destroy_request(struct wl_client* client, struct wl_resource* resource)
{
    (void)client;
    wl_resource_destroy(resource);
}
static void frame_destroyed(struct wl_resource* resource)
{
    struct frame* f = wl_resource_get_user_data(resource);
    ++f->server->frames_destroyed;
    free(f);
}
static void copy(struct wl_client* client, struct wl_resource* resource, struct wl_resource* buffer)
{
    (void)client;
    struct frame* f = wl_resource_get_user_data(resource);
    struct wl_shm_buffer* shm = wl_shm_buffer_get(buffer);
    if (f->copied || !shm || wl_shm_buffer_get_width(shm) != 7 || wl_shm_buffer_get_height(shm) != 5
        || wl_shm_buffer_get_format(shm) != f->input.format
        || wl_shm_buffer_get_stride(shm) != (int32_t)f->input.stride) {
        failure(f->server,"actual native SHM input differs");
        return;
    }
    f->copied = true;
    if (f->server->failures) {
        const uint32_t buffer_id = wl_resource_get_id(buffer);
        if (f->cycle >= 20) { failure(f->server,"invalid failure frame cycle"); return; }
        if (f->phase == 1) { f->server->failure_buffers[f->cycle] = buffer_id; }
        if (!buffer_id || f->server->failure_buffers[f->cycle] != buffer_id) { failure(f->server,"failure path replaced unchanged SHM buffer"); return; }
        if (fprintf(f->server->failure_rows,"%u\t%u\t%u\t%u\t%u\n",f->cycle,f->phase,
            wl_resource_get_id(resource),buffer_id,f->phase == 2 ? 0u : 1u) < 0
            || fflush(f->server->failure_rows) != 0) { failure(f->server,"save failure copy request"); return; }
    }
    // The real libwayland-server SHM helper owns the mapped received FD.
    // Access ends before Ready; no pointer/reference escapes this callback.
    wl_shm_buffer_begin_access(shm);
    uint8_t* bytes = wl_shm_buffer_get_data(shm);
    mfs_protocol_fill(f->input,bytes);
    if (f->server->failures || f->server->registry || f->server->copies < 24) {
        char name[4096];
        int length = f->server->failures
            ? snprintf(name,sizeof name,"%s/case-%u-%u.shm",f->server->directory,f->cycle,f->phase)
            : snprintf(name,sizeof name,"%s/case-%u.shm",f->server->directory,
                f->server->registry ? f->server->copies : f->input.index);
        FILE* file = length > 0 && (size_t)length < sizeof name ? fopen(name,"wb") : NULL;
        const size_t size = 5 * f->input.stride;
        bool saved = file && fwrite(bytes,1,size,file) == size;
        if (file && fclose(file) != 0) { saved = false; }
        if (!saved) { wl_shm_buffer_end_access(shm); failure(f->server,"save actual producer input"); return; }
    }
    wl_shm_buffer_end_access(shm);
    ++f->server->copies;
    if (f->server->failures && f->phase == 2) {
        ++f->server->failed_frames;
        zwlr_screencopy_frame_v1_send_failed(resource);
        return;
    }
    zwlr_screencopy_frame_v1_send_flags(resource,f->input.inverted);
    zwlr_screencopy_frame_v1_send_ready(resource,0,2000 + f->input.index,1000 + f->input.index);
}
static const struct zwlr_screencopy_frame_v1_interface frame_requests = {
    .copy = copy, .destroy = destroy_request, .copy_with_damage = copy,
};
static void capture(struct wl_client* client, struct wl_resource* resource, uint32_t id,
    int32_t overlay_cursor, struct wl_resource* output)
{
    struct manager* m = wl_resource_get_user_data(resource);
    struct server* s = m->server;
    if (m->next >= (s->failures ? 5u : s->registry ? 80u : 24u) || overlay_cursor != 0 || !output) { failure(s,"unexpected capture request"); return; }
    const unsigned position = m->next++;
    unsigned input = position;
    if (s->failures) {
        if (m->cycle >= 20) { failure(s,"too many failure clients"); return; }
        if (position == 0 || position == 4) {
            if (fprintf(s->failure_rows,"%u\t%u\t%u\t0\t%u\n",m->cycle,position,id,position == 4 ? 2u : 0u) < 0
                || fflush(s->failure_rows) != 0) { failure(s,"save failure request"); return; }
        }
        if (position == 4) {
            // Deliberate EOF while the real client waits for this request. Do
            // not dereference m/resource after destroying their native owner.
            ++s->disconnected_clients;
            wl_client_destroy(client);
            return;
        }
        input = position < 2 ? 0 : 4;
    }
    if (s->registry) {
        struct registry_output* selected = wl_resource_get_user_data(output);
        const unsigned round = position / 4, phase = position % 4;
        const unsigned expected = round + (phase < 2 ? 1 : 2);
        if (!selected || selected->server != s || selected->generation != expected) {
            failure(s,"registry output selection or retained snapshot differs"); return;
        }
        input = selected->generation % 2 ? 0 : 4;
        if (fprintf(s->registry_rows,"%u\t%u\t%u\t%u\t%u\n",position,selected->generation,
            wl_resource_get_id(output),input,wl_resource_get_version(output)) < 0
            || fflush(s->registry_rows) != 0) { failure(s,"save actual registry selection"); return; }
    }
    struct frame* f = calloc(1,sizeof *f);
    if (!f) { wl_client_post_no_memory(client); failure(m->server,"frame allocation"); return; }
    f->server = m->server;
    f->input = mfs_protocol_case(input);
    f->cycle = m->cycle; f->phase = position;
    if (s->failures) { f->input.format = WL_SHM_FORMAT_XRGB8888; }
    struct wl_resource* frame = wl_resource_create(client,&zwlr_screencopy_frame_v1_interface,
        wl_resource_get_version(resource),id);
    if (!frame) { free(f); wl_client_post_no_memory(client); failure(m->server,"frame resource"); return; }
    ++m->server->frames_created;
    wl_resource_set_implementation(frame,&frame_requests,f,frame_destroyed);
    if (s->failures && position == 0) {
        ++s->failed_frames;
        zwlr_screencopy_frame_v1_send_failed(frame);
        return;
    }
    zwlr_screencopy_frame_v1_send_buffer(frame,f->input.format,7,5,f->input.stride);
    if (wl_resource_get_version(frame) >= 3) { zwlr_screencopy_frame_v1_send_buffer_done(frame); }
    if (s->registry) { (void)registry_advance(s,position % 4,position / 4); }
}
static void capture_region(struct wl_client* client, struct wl_resource* resource, uint32_t id,
    int32_t overlay, struct wl_resource* output, int32_t x, int32_t y, int32_t w, int32_t h)
{
    (void)client; (void)id; (void)overlay; (void)output; (void)x; (void)y; (void)w; (void)h;
    struct manager* m = wl_resource_get_user_data(resource);
    failure(m->server,"unexpected region request");
}
static const struct zwlr_screencopy_manager_v1_interface manager_requests = {
    .capture_output = capture, .capture_output_region = capture_region, .destroy = destroy_request,
};
static void manager_destroyed(struct wl_resource* resource)
{
    struct manager* m = wl_resource_get_user_data(resource);
    ++m->server->managers_destroyed;
    free(m);
}
static void bind_manager(struct wl_client* client, void* data, uint32_t version, uint32_t id)
{
    struct server* s = data;
    struct manager* m = calloc(1,sizeof *m);
    if (!m) { wl_client_post_no_memory(client); failure(s,"manager allocation"); return; }
    m->server = s;
    m->cycle = s->managers_created;
    struct wl_resource* resource = wl_resource_create(client,&zwlr_screencopy_manager_v1_interface,version,id);
    if (!resource) { free(m); wl_client_post_no_memory(client); failure(s,"manager resource"); return; }
    ++s->managers_created;
    wl_resource_set_implementation(resource,&manager_requests,m,manager_destroyed);
}
static void deny_pointer(struct wl_client* c, struct wl_resource* r, struct wl_resource* seat, uint32_t id)
{
    (void)c; (void)seat; (void)id;
    wl_resource_post_error(r,0,"input requests are outside the owned capture fixture");
}
static void deny_pointer_output(struct wl_client* c, struct wl_resource* r, struct wl_resource* seat,
    struct wl_resource* output, uint32_t id)
{
    (void)output;
    deny_pointer(c,r,seat,id);
}
static const struct zwlr_virtual_pointer_manager_v1_interface pointer_requests = {
    .create_virtual_pointer = deny_pointer, .destroy = destroy_request,
    .create_virtual_pointer_with_output = deny_pointer_output,
};
static void deny_keyboard(struct wl_client* c, struct wl_resource* r, struct wl_resource* seat, uint32_t id)
{
    deny_pointer(c,r,seat,id);
}
static const struct zwp_virtual_keyboard_manager_v1_interface keyboard_requests = {
    .create_virtual_keyboard = deny_keyboard,
};
static const struct wl_output_interface output_requests = { .release = destroy_request };
static void bind_pointer(struct wl_client* c, void* data, uint32_t version, uint32_t id)
{
    struct wl_resource* r = wl_resource_create(c,&zwlr_virtual_pointer_manager_v1_interface,version,id);
    if (r) { wl_resource_set_implementation(r,&pointer_requests,data,NULL); }
    else { wl_client_post_no_memory(c); failure(data,"passive pointer resource"); }
}
static void bind_keyboard(struct wl_client* c, void* data, uint32_t version, uint32_t id)
{
    struct wl_resource* r = wl_resource_create(c,&zwp_virtual_keyboard_manager_v1_interface,version,id);
    if (r) { wl_resource_set_implementation(r,&keyboard_requests,data,NULL); }
    else { wl_client_post_no_memory(c); failure(data,"passive keyboard resource"); }
}
static void bind_output(struct wl_client* c, void* data, uint32_t version, uint32_t id)
{
    struct wl_resource* r = wl_resource_create(c,&wl_output_interface,version,id);
    if (r) { wl_resource_set_implementation(r,&output_requests,data,NULL); }
    else { wl_client_post_no_memory(c); failure(data,"passive output resource"); }
}
static void registry_output_destroyed(struct wl_resource* r)
{
    struct registry_output* output = wl_resource_get_user_data(r);
    ++output->server->output_destroys;
}
static void bind_registry_output(struct wl_client* c, void* data, uint32_t version, uint32_t id)
{
    struct registry_output* output = data;
    struct wl_resource* r = wl_resource_create(c,&wl_output_interface,version,id);
    if (!r) { wl_client_post_no_memory(c); failure(output->server,"registry output resource"); return; }
    ++output->server->output_binds;
    wl_resource_set_implementation(r,&output_requests,output,registry_output_destroyed);
}
static bool registry_add(struct server* s, unsigned generation)
{
    if (generation >= 22 || s->outputs[generation].global) { failure(s,"registry generation overflow"); return false; }
    struct registry_output* output = &s->outputs[generation];
    output->server = s; output->generation = generation;
    // Alternate actual advertised versions. U binds the advertised version;
    // version 1/2 use local proxy destruction, without output release requests.
    output->global = wl_global_create(s->display,&wl_output_interface,1 + generation % 2,output,bind_registry_output);
    if (!output->global) { failure(s,"registry output global"); return false; }
    ++s->output_globals;
    return true;
}
static bool registry_advance(struct server* s, unsigned phase, unsigned round)
{
    if (phase == 0) {
        if (!registry_add(s,round + 2)) { return false; }
        if (round == 0) {
            wl_global_destroy(s->outputs[1].global); s->outputs[1].global = NULL; ++s->output_removed;
        }
    } else if (phase == 2) {
        struct registry_output* output = &s->outputs[round + 2];
        if (!output->global) { failure(s,"registry output already removed"); return false; }
        wl_global_destroy(output->global); output->global = NULL; ++s->output_removed;
    }
    return true;
}
static int stop(int signal, void* data)
{
    (void)signal;
    struct server* s = data;
    s->stopped = true;
    wl_display_terminate(s->display);
    return 0;
}
int main(int argc, char** argv)
{
    if ((argc != 4 && argc != 5) || (argc == 5 && strcmp(argv[4],"--registry") != 0 && strcmp(argv[4],"--failures") != 0)
        || strlen(argv[1]) != 1 || argv[1][0] < '1' || argv[1][0] > '3') { return 64; }
    Dl_info owner;
    char actual[PATH_MAX],expected[PATH_MAX];
    if (!dladdr((void*)&wl_display_create,&owner) || !owner.dli_fname
        || !realpath(owner.dli_fname,actual) || !realpath(argv[3],expected) || strcmp(actual,expected) != 0) {
        fprintf(stderr,"protocol fixture loaded a different native server library\n");
        return 1;
    }
    printf("native protocol server library=%s\n",actual);
    struct server s = {.directory = argv[2], .version = (unsigned)(argv[1][0] - '0'),
        .registry = argc == 5 && strcmp(argv[4],"--registry") == 0,
        .failures = argc == 5 && strcmp(argv[4],"--failures") == 0};
    s.display = wl_display_create();
    if (!s.display) { return 1; }
    struct wl_event_loop* loop = wl_display_get_event_loop(s.display);
    struct wl_event_source* term = wl_event_loop_add_signal(loop,SIGTERM,stop,&s);
    struct wl_event_source* interrupt = wl_event_loop_add_signal(loop,SIGINT,stop,&s);
    if (!term || !interrupt || zwlr_screencopy_manager_v1_interface.version != 3
        || wl_display_init_shm(s.display) != 0) { s.error = 1; goto cleanup; }
    for (uint32_t kind = 2; kind < 6; ++kind) {
        if (!wl_display_add_shm_format(s.display,mfs_protocol_case(kind * 4).format)) { s.error = 1; goto cleanup; }
    }
    if (s.failures) {
        char name[4096];
        int length = snprintf(name,sizeof name,"%s/failure-requests.tsv",s.directory);
        s.failure_rows = length > 0 && (size_t)length < sizeof name ? fopen(name,"wx") : NULL;
        if (!s.failure_rows) { s.error = 1; goto cleanup; }
    }
    if (s.registry) {
        char name[4096];
        int length = snprintf(name,sizeof name,"%s/registry-requests.tsv",s.directory);
        s.registry_rows = length > 0 && (size_t)length < sizeof name ? fopen(name,"wx") : NULL;
        if (!s.registry_rows || !registry_add(&s,0) || !registry_add(&s,1)) { s.error = 1; goto cleanup; }
    } else if (!wl_global_create(s.display,&wl_output_interface,1,&s,bind_output)) { s.error = 1; goto cleanup; }
    if (!wl_global_create(s.display,&zwlr_virtual_pointer_manager_v1_interface,1,&s,bind_pointer)
        || !wl_global_create(s.display,&zwp_virtual_keyboard_manager_v1_interface,1,&s,bind_keyboard)
        || !wl_global_create(s.display,&zwlr_screencopy_manager_v1_interface,(int)s.version,&s,bind_manager)
        || wl_display_add_socket(s.display,"mfs-p1-protocol") != 0) { s.error = 1; goto cleanup; }
    if (s.failures) { printf("failures ready cycles=20 operations=100 scripted_failed=40 disconnects=20 compositor=0\n"); }
    else if (s.registry) { printf("registry ready rounds=20 captures=80 initial_outputs=2 compositor=0\n"); }
    else { printf("protocol ready version=%u formats=6 cases=24 native_shm=1 compositor=0\n",s.version); }
    fflush(stdout);
    wl_display_run(s.display);
cleanup:
    wl_display_destroy_clients(s.display);
    if (term) { wl_event_source_remove(term); }
    if (interrupt) { wl_event_source_remove(interrupt); }
    wl_display_destroy(s.display);
    if (s.registry_rows && fclose(s.registry_rows) != 0) { s.error = 1; }
    if (s.failure_rows && fclose(s.failure_rows) != 0) { s.error = 1; }
    const unsigned managers = s.failures ? 20 : s.registry ? 1 : 40;
    const unsigned frames = s.failures || s.registry ? 80 : 960, copies = s.failures ? 60 : frames;
    if (!s.stopped || s.error || s.managers_created != managers || s.managers_destroyed != managers
        || s.frames_created != frames || s.frames_destroyed != frames || s.copies != copies) { s.error = 1; }
    if (s.failures) {
        if (s.failed_frames != 40 || s.disconnected_clients != 20) { s.error = 1; }
        printf("failures stopped clients=%u/%u frames=%u/%u copies=%u failed=%u disconnected=%u graceful=%d error=%d\n",
            s.managers_created,s.managers_destroyed,s.frames_created,s.frames_destroyed,s.copies,
            s.failed_frames,s.disconnected_clients,s.stopped,s.error);
    }
    if (s.registry) {
        if (s.output_globals != 22 || s.output_removed != 21 || s.output_binds != 22 || s.output_destroys != 22) { s.error = 1; }
        printf("registry stopped rounds=20 globals=%u removed=%u outputs=%u/%u frames=%u/%u copies=%u graceful=%d error=%d\n",
            s.output_globals,s.output_removed,s.output_binds,s.output_destroys,s.frames_created,s.frames_destroyed,s.copies,s.stopped,s.error);
    }
    printf("protocol stopped version=%u managers=%u/%u frames=%u/%u copies=%u graceful=%d error=%d\n",
        s.version,s.managers_created,s.managers_destroyed,s.frames_created,s.frames_destroyed,s.copies,s.stopped,s.error);
    return s.error;
}
