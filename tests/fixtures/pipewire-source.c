// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

// Owned producer. Default P0 bars and explicit P1 asymmetric RGB rows carry
// initialized padding across the private remote to the independent Rust client.
#include <inttypes.h>
#include <signal.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <pipewire/pipewire.h>
#include <spa/param/video/format-utils.h>
#include "pipewire-format-pattern.h"

enum { WIDTH = 28, HEIGHT = 24, STRIDE = 96, FRAME_BYTES = STRIDE * HEIGHT };
struct producer {
    struct pw_main_loop *loop;
    struct pw_stream *stream;
    struct spa_source *timer;
    struct spa_hook listener;
    int error;
    bool stopping;
    bool asymmetric;
    bool format_stream;
    enum spa_video_format format;
    unsigned channels, stride, frame_bytes;
    uint64_t phases[2];
    uint64_t dequeued, queued;
};

static void fail(struct producer *p, const char *message)
{
    fprintf(stderr, "%s\n", message);
    p->error = 1;
    pw_main_loop_quit(p->loop);
}

static void produce(void *userdata)
{
    struct producer *p = userdata;
    struct pw_buffer *lease = pw_stream_dequeue_buffer(p->stream);
    if (lease == NULL)
        return;
    p->dequeued++;
    struct spa_buffer *buffer = lease->buffer;
    bool valid = buffer != NULL && buffer->n_datas == 1 && buffer->datas != NULL;
    struct spa_data *plane = valid ? &buffer->datas[0] : NULL;
    valid = valid && plane->data != NULL && plane->chunk != NULL &&
        plane->maxsize >= p->frame_bytes && (plane->flags & SPA_DATA_FLAG_WRITABLE) != 0 &&
        (plane->type == SPA_DATA_MemFd || plane->type == SPA_DATA_MemPtr);
    if (valid) {
        static const unsigned char bars[7][3] = {
            {191,191,191}, {191,191,0}, {0,191,191}, {0,191,0},
            {191,0,191}, {191,0,0}, {0,0,191}
        };
        unsigned char *pixels = plane->data;
        if (p->format_stream) {
            unsigned phase = (unsigned)(p->dequeued & 1);
            mfs_pw_format_pattern(pixels, p->channels, p->stride, phase);
            p->phases[phase]++;
        } else {
            memset(pixels, 0xa5, FRAME_BYTES);
            for (size_t y = 0; y < HEIGHT; y++)
                for (size_t x = 0; x < WIDTH; x++) {
                    memcpy(pixels + y * STRIDE + x * 3, bars[x / 4], 3);
                    if (p->asymmetric) {
                        for (size_t c = 0; c < 3; c++)
                            pixels[y * STRIDE + x * 3 + c] ^= (unsigned char)((y * 7 + x * 3 + c * 13 + (p->dequeued & 63)) & 63);
                    }
                }
            if (p->asymmetric) {
                const uint32_t marker = (uint32_t)(p->dequeued & 0xffffff);
                pixels[0] = (unsigned char)(marker >> 16);
                pixels[1] = (unsigned char)(marker >> 8);
                pixels[2] = (unsigned char)marker;
            }
        }
        *plane->chunk = (struct spa_chunk){ .offset = 0, .size = p->frame_bytes, .stride = (int32_t)p->stride };
    }
    // Every successful dequeue has exactly one return, including failure paths.
    if (pw_stream_queue_buffer(p->stream, lease) < 0)
        fail(p, "producer buffer return failed");
    else
        p->queued++;
    if (!valid)
        fail(p, "producer received an invalid writable native layout");
}

static void tick(void *userdata, uint64_t expirations)
{
    (void)expirations;
    struct producer *p = userdata;
    if (pw_stream_trigger_process(p->stream) < 0)
        fail(p, "producer trigger failed");
}

static void state_changed(void *userdata, enum pw_stream_state old,
                          enum pw_stream_state state, const char *error)
{
    (void)old;
    struct producer *p = userdata;
    if (state == PW_STREAM_STATE_ERROR) {
        fail(p, error != NULL ? error : "producer stream failed");
    } else if (state == PW_STREAM_STATE_STREAMING) {
        struct timespec start = { .tv_nsec = 1 };
        struct timespec interval = { .tv_nsec = 40000000 };
        if (pw_loop_update_timer(pw_main_loop_get_loop(p->loop), p->timer,
                                 &start, &interval, false) < 0)
            fail(p, "producer timer start failed");
    } else if (state == PW_STREAM_STATE_PAUSED) {
        if (pw_loop_update_timer(pw_main_loop_get_loop(p->loop), p->timer,
                                 NULL, NULL, false) < 0)
            fail(p, "producer timer pause failed");
    }
}

static void format_changed(void *userdata, uint32_t id, const struct spa_pod *param)
{
    struct producer *p = userdata;
    if (id != SPA_PARAM_Format || param == NULL)
        return;
    struct spa_video_info_raw format = {0};
    if (spa_format_video_raw_parse(param, &format) < 0 ||
        format.format != p->format || format.size.width != WIDTH ||
        format.size.height != HEIGHT) {
        fail(p, "producer negotiated an unexpected format");
        return;
    }
    unsigned char storage[512];
    struct spa_pod_builder builder = SPA_POD_BUILDER_INIT(storage, sizeof(storage));
    const struct spa_pod *buffers = spa_pod_builder_add_object(&builder,
        SPA_TYPE_OBJECT_ParamBuffers, SPA_PARAM_Buffers,
        SPA_PARAM_BUFFERS_buffers, SPA_POD_CHOICE_RANGE_Int(8, 2, 16),
        SPA_PARAM_BUFFERS_blocks, SPA_POD_Int(1),
        SPA_PARAM_BUFFERS_size, SPA_POD_Int((int32_t)p->frame_bytes),
        SPA_PARAM_BUFFERS_stride, SPA_POD_Int((int32_t)p->stride));
    if (buffers == NULL || pw_stream_update_params(p->stream, &buffers, 1) < 0)
        fail(p, "producer buffer negotiation failed");
}

static void stop(void *userdata, int number)
{
    (void)number;
    struct producer *p = userdata;
    p->stopping = true;
    pw_main_loop_quit(p->loop);
}

int main(int argc, char **argv)
{
    struct producer p = { .format = SPA_VIDEO_FORMAT_RGB, .channels = 3,
                          .stride = STRIDE, .frame_bytes = FRAME_BYTES };
    if (argc == 2 && strcmp(argv[1], "--asymmetric") == 0) {
        p.asymmetric = true;
        argc = 1;
        argv[1] = NULL;
    } else if (argc == 3 && strcmp(argv[1], "--format") == 0) {
        static const struct { const char *name; enum spa_video_format format; unsigned channels; } formats[] = {
            {"RGB", SPA_VIDEO_FORMAT_RGB, 3}, {"RGBA", SPA_VIDEO_FORMAT_RGBA, 4},
            {"RGBx", SPA_VIDEO_FORMAT_RGBx, 4}, {"BGR", SPA_VIDEO_FORMAT_BGR, 3},
            {"BGRA", SPA_VIDEO_FORMAT_BGRA, 4}, {"BGRx", SPA_VIDEO_FORMAT_BGRx, 4},
            {"YUY2", SPA_VIDEO_FORMAT_YUY2, 2}
        };
        for (size_t i = 0; i < sizeof(formats) / sizeof(formats[0]); ++i) {
            if (strcmp(argv[2], formats[i].name) == 0) {
                p.format_stream = true; p.format = formats[i].format; p.channels = formats[i].channels;
            }
        }
        if (!p.format_stream) { fprintf(stderr, "unknown explicit format\n"); return 1; }
        p.stride = WIDTH * p.channels + 12;
        p.frame_bytes = p.stride * HEIGHT;
        argc = 1; argv[1] = NULL;
    } else if (argc != 1) {
        fprintf(stderr, "owned source accepts --asymmetric or --format NAME\n");
        return 1;
    }
    struct pw_context *context = NULL;
    struct pw_core *core = NULL;
    pw_init(&argc, &argv);
    p.loop = pw_main_loop_new(NULL);
    if (p.loop == NULL) { pw_deinit(); return 1; }
    struct pw_loop *loop = pw_main_loop_get_loop(p.loop);
    p.timer = pw_loop_add_timer(loop, tick, &p);
    if (p.timer == NULL || pw_loop_add_signal(loop, SIGTERM, stop, &p) == NULL ||
        pw_loop_add_signal(loop, SIGINT, stop, &p) == NULL) goto failed;
    context = pw_context_new(loop, NULL, 0);
    if (context == NULL) goto failed;
    core = pw_context_connect(context, NULL, 0);
    if (core == NULL) goto failed;
    p.stream = pw_stream_new(core, "mfs-p0-owned-source", pw_properties_new(
        PW_KEY_NODE_NAME, "mfs-p0-fixture", PW_KEY_MEDIA_CLASS, "Video/Source",
        PW_KEY_NODE_SUPPORTS_REQUEST, "1", NULL));
    if (p.stream == NULL) goto failed;
    static const struct pw_stream_events events = {
        PW_VERSION_STREAM_EVENTS, .process = produce,
        .state_changed = state_changed, .param_changed = format_changed
    };
    pw_stream_add_listener(p.stream, &p.listener, &events, &p);
    unsigned char storage[512];
    struct spa_pod_builder builder = SPA_POD_BUILDER_INIT(storage, sizeof(storage));
    const struct spa_pod *format = spa_pod_builder_add_object(&builder,
        SPA_TYPE_OBJECT_Format, SPA_PARAM_EnumFormat,
        SPA_FORMAT_mediaType, SPA_POD_Id(SPA_MEDIA_TYPE_video),
        SPA_FORMAT_mediaSubtype, SPA_POD_Id(SPA_MEDIA_SUBTYPE_raw),
        SPA_FORMAT_VIDEO_format, SPA_POD_Id(p.format),
        SPA_FORMAT_VIDEO_size, SPA_POD_Rectangle(&SPA_RECTANGLE(WIDTH, HEIGHT)),
        SPA_FORMAT_VIDEO_framerate, SPA_POD_Fraction(&SPA_FRACTION(25, 1)));
    if (format == NULL || pw_stream_connect(p.stream, PW_DIRECTION_OUTPUT, PW_ID_ANY,
        PW_STREAM_FLAG_DRIVER | PW_STREAM_FLAG_MAP_BUFFERS, &format, 1) < 0) goto failed;
    if (pw_main_loop_run(p.loop) < 0) p.error = 1;
    if (!p.stopping || p.dequeued == 0 || p.dequeued != p.queued) p.error = 1;
    goto cleanup;
failed:
    p.error = 1;
cleanup:
    if (p.timer != NULL) pw_loop_destroy_source(loop, p.timer);
    if (p.stream != NULL) {
        spa_hook_remove(&p.listener);
        if (pw_stream_disconnect(p.stream) < 0) p.error = 1;
        pw_stream_destroy(p.stream);
    }
    if (core != NULL && pw_core_disconnect(core) < 0) p.error = 1;
    if (context != NULL) pw_context_destroy(context);
    pw_main_loop_destroy(p.loop);
    pw_deinit();
    printf("owned producer dequeued=%" PRIu64 ", queued=%" PRIu64 ", error=%d\n",
           p.dequeued, p.queued, p.error);
    if (p.format_stream) {
        if (p.phases[0] == 0 || p.phases[1] == 0) p.error = 1;
        printf("format producer format=%u channels=%u stride=%u phases=%" PRIu64 ",%" PRIu64 " error=%d\n",
               p.format, p.channels, p.stride, p.phases[0], p.phases[1], p.error);
    }
    return p.error;
}
