// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

// P0 owned native compositor: real wlroots rendering and screencopy, no input,
// host desktop connection, GPU fallback or product backend implementation.
#include <signal.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <time.h>
#include <wayland-server-core.h>
#include <wlr/backend/headless.h>
#include <wlr/render/allocator.h>
#include <wlr/render/pixman.h>
#include <wlr/types/wlr_compositor.h>
#include <wlr/types/wlr_scene.h>
#include <wlr/types/wlr_screencopy_v1.h>
#include <wlr/types/wlr_shm.h>
#include <wlr/types/wlr_virtual_keyboard_v1.h>
#include <wlr/types/wlr_virtual_pointer_v1.h>
#include <wlr/util/log.h>

struct fixture {
    struct wl_display *display;
    struct wlr_backend *backend;
    struct wlr_renderer *renderer;
    struct wlr_allocator *allocator;
    struct wlr_scene *scene;
    struct wlr_scene_output *scene_output;
    struct wlr_output *output;
    struct wl_listener frame;
    bool frame_attached, stopped;
    unsigned frames;
    int error;
};

static int stop(int number, void *userdata)
{
    (void)number;
    struct fixture *f = userdata;
    f->stopped = true;
    wl_display_terminate(f->display);
    return 0;
}

static void render(struct wl_listener *listener, void *unused)
{
    (void)unused;
    struct fixture *f = wl_container_of(listener, f, frame);
    if (!wlr_scene_output_commit(f->scene_output, NULL)) {
        f->error = 1;
        wl_display_terminate(f->display);
        return;
    }
    f->frames++;
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) {
        f->error = 1;
        wl_display_terminate(f->display);
        return;
    }
    wlr_scene_output_send_frame_done(f->scene_output, &now);
}

int main(int argc, char **argv)
{
    const bool source_client = argc == 2 && strcmp(argv[1], "--source-client") == 0;
    if (argc != 1 && !source_client) return 1;
    struct fixture f = {0};
    struct wl_event_source *term = NULL, *interrupt = NULL;
    wlr_log_init(WLR_INFO, NULL);
    f.display = wl_display_create();
    if (f.display == NULL) return 1;
    struct wl_event_loop *loop = wl_display_get_event_loop(f.display);
    term = wl_event_loop_add_signal(loop, SIGTERM, stop, &f);
    interrupt = wl_event_loop_add_signal(loop, SIGINT, stop, &f);
    if (term == NULL || interrupt == NULL) goto failed;
    f.backend = wlr_headless_backend_create(loop);
    // Explicit software renderer; never retry another renderer on failure.
    f.renderer = wlr_pixman_renderer_create();
    if (f.backend == NULL || f.renderer == NULL) goto failed;
    f.allocator = wlr_allocator_autocreate(f.backend, f.renderer);
    f.scene = wlr_scene_create();
    if (f.allocator == NULL || f.scene == NULL) goto failed;
    if (wlr_compositor_create(f.display, 6, f.renderer) == NULL ||
        wlr_shm_create_with_renderer(f.display, 1, f.renderer) == NULL ||
        wlr_screencopy_manager_v1_create(f.display) == NULL) goto failed;
    // The fixed original WaylandClient insists on these passive globals even
    // for screenshots. This explicit P1 mode never creates or handles input
    // devices, does not connect a host seat, and leaves the P0 mode unchanged.
    if (source_client && (wlr_virtual_keyboard_manager_v1_create(f.display) == NULL ||
        wlr_virtual_pointer_manager_v1_create(f.display) == NULL)) goto failed;
    const float blue[4] = {0,0,1,1}, red[4] = {1,0,0,1}, green[4] = {0,1,0,1};
    if (wlr_scene_rect_create(&f.scene->tree, 64, 48, blue) == NULL ||
        wlr_scene_rect_create(&f.scene->tree, 32, 24, red) == NULL) goto failed;
    struct wlr_scene_rect *right = wlr_scene_rect_create(&f.scene->tree, 32, 24, green);
    if (right == NULL) goto failed;
    wlr_scene_node_set_position(&right->node, 32, 0);
    if (!wlr_backend_start(f.backend)) goto failed;
    f.output = wlr_headless_add_output(f.backend, 64, 48);
    if (f.output == NULL || !wlr_output_init_render(f.output, f.allocator, f.renderer)) goto failed;
    wlr_output_set_name(f.output, "mfs-p0-output");
    f.scene_output = wlr_scene_output_create(f.scene, f.output);
    if (f.scene_output == NULL) goto failed;
    f.frame.notify = render;
    wl_signal_add(&f.output->events.frame, &f.frame);
    f.frame_attached = true;
    struct wlr_output_state state;
    wlr_output_state_init(&state);
    wlr_output_state_set_enabled(&state, true);
    wlr_output_state_set_custom_mode(&state, 64, 48, 60000);
    bool configured = wlr_output_commit_state(f.output, &state);
    wlr_output_state_finish(&state);
    if (!configured) goto failed;
    wlr_output_create_global(f.output, f.display);
    // Socket readiness is published only after the full native scene is ready.
    if (wl_display_add_socket(f.display, "mfs-p0-wlr") != 0) goto failed;
    printf("ready: headless output=64x48 renderer=pixman\n");
    fflush(stdout);
    wl_display_run(f.display);
    if (!f.stopped || f.frames == 0) f.error = 1;
    goto cleanup;
failed:
    f.error = 1;
cleanup:
    wl_display_destroy_clients(f.display);
    if (f.frame_attached) wl_list_remove(&f.frame.link);
    if (f.scene_output != NULL) wlr_scene_output_destroy(f.scene_output);
    if (f.scene != NULL) wlr_scene_node_destroy(&f.scene->tree.node);
    if (f.backend != NULL) wlr_backend_destroy(f.backend);
    if (f.allocator != NULL) wlr_allocator_destroy(f.allocator);
    if (f.renderer != NULL) wlr_renderer_destroy(f.renderer);
    if (term != NULL) wl_event_source_remove(term);
    if (interrupt != NULL) wl_event_source_remove(interrupt);
    wl_display_destroy(f.display);
    printf("native compositor frames=%u, graceful=%d, error=%d\n", f.frames, f.stopped, f.error);
    return f.error;
}
