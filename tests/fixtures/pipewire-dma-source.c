// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Owned CPU-written kernel DMA-BUF producer. This is not a GPU/Portal fixture.
// ALLOC_BUFFERS supplies real DMA descriptors without MAPPABLE; the consumer
// retains its original MAP_BUFFERS flags and must map a NULL incoming pointer.
#include "dma-fixture.h"
#include <errno.h>
#include <inttypes.h>
#include <signal.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <pipewire/pipewire.h>
#include <spa/param/video/format-utils.h>

enum { WIDTH = 28, HEIGHT = 24, STRIDE = 96, FRAME_BYTES = STRIDE * HEIGHT };
struct producer {
    struct pw_main_loop *loop;
    struct pw_stream *stream;
    struct spa_source *timer;
    struct spa_hook listener;
    uint32_t exporter;
    int error;
    bool stopping;
    uint64_t dequeued, queued, allocated, removed, writes;
    struct { struct pw_buffer *buffer; int fd; } slots[16];
};
static void fail(struct producer *p, const char *message)
{
    fprintf(stderr, "%s errno=%d\n", message, errno);
    p->error = 1;
    pw_main_loop_quit(p->loop);
}
static void add_buffer(void *data, struct pw_buffer *lease)
{
    struct producer *p = data;
    struct spa_buffer *buffer = lease->buffer;
    if (!buffer || buffer->n_datas != 1 || !buffer->datas ||
        !buffer->datas[0].chunk || (buffer->datas[0].type & (1u << SPA_DATA_DmaBuf)) == 0) {
        fail(p, "DMA allocation negotiation did not offer one DmaBuf plane");
        return;
    }
    struct spa_data *plane = &buffer->datas[0];
    size_t slot = 0;
    while (slot < 16 && p->slots[slot].buffer) slot++;
    if (slot == 16) { fail(p, "DMA allocation exceeded negotiated capacity"); return; }
    plane->type = SPA_DATA_DmaBuf;
    plane->flags = SPA_DATA_FLAG_READWRITE; // deliberately no MAPPABLE
    plane->data = NULL;
    plane->fd = -1;
    plane->mapoffset = 0;
    plane->maxsize = 4096;
    const int fd = mfs_pw_dma_create(p->exporter, plane->maxsize);
    if (fd < 0) { errno = -fd; fail(p, "kernel DMA allocation failed"); return; }
    plane->fd = fd;
    p->slots[slot].buffer = lease;
    p->slots[slot].fd = fd;
    p->allocated++;
    printf("DMA add fd=%d type=%u flags=%u data_null=1 bytes=%u\n",
           fd, plane->type, plane->flags, plane->maxsize);
}
static void remove_buffer(void *data, struct pw_buffer *lease)
{
    struct producer *p = data;
    for (size_t i = 0; i < 16; i++) {
        if (p->slots[i].buffer == lease) {
            if (close(p->slots[i].fd) < 0) fail(p, "DMA descriptor close failed");
            p->slots[i].buffer = NULL;
            p->removed++;
            return;
        }
    }
    // An add_buffer that failed before allocation has no owned FD to close.
    if (!p->error) fail(p, "DMA removal has no matching owned allocation");
}
static void produce(void *data)
{
    struct producer *p = data;
    struct pw_buffer *lease = pw_stream_dequeue_buffer(p->stream);
    if (!lease) return;
    p->dequeued++;
    struct spa_buffer *buffer = lease->buffer;
    struct spa_data *plane = buffer && buffer->n_datas == 1 && buffer->datas ? &buffer->datas[0] : NULL;
    bool valid = plane && plane->chunk && plane->type == SPA_DATA_DmaBuf &&
        plane->fd >= 0 && plane->fd <= INT32_MAX && plane->data == NULL &&
        plane->mapoffset == 0 && plane->maxsize == 4096 &&
        plane->flags == SPA_DATA_FLAG_READWRITE;
    if (valid) {
        static const unsigned char bars[7][3] = {
            {191,191,191}, {191,191,0}, {0,191,191}, {0,191,0},
            {191,0,191}, {191,0,0}, {0,0,191}
        };
        unsigned char pixels[4096];
        memset(pixels, 0xa5, sizeof(pixels));
        const uint32_t marker = (uint32_t)(p->dequeued & 0xffffff);
        for (size_t y = 0; y < HEIGHT; y++)
            for (size_t x = 0; x < WIDTH; x++)
                for (size_t c = 0; c < 3; c++)
                    pixels[y * STRIDE + x * 3 + c] = bars[x / 4][c] ^
                        (unsigned char)((y * 7 + x * 3 + c * 13 + (marker & 63)) & 63);
        pixels[0] = (unsigned char)(marker >> 16);
        pixels[1] = (unsigned char)(marker >> 8);
        pixels[2] = (unsigned char)marker;
        const int error = mfs_pw_dma_write((int32_t)plane->fd, pixels, sizeof(pixels));
        // The synchronous WRITE START/copy/END all finish before queue_buffer.
        if (error) { errno = error; valid = false; fail(p, "DMA producer WRITE access failed"); }
        else p->writes++;
        *plane->chunk = (struct spa_chunk){ .offset = 0, .size = FRAME_BYTES, .stride = STRIDE };
    }
    if (pw_stream_queue_buffer(p->stream, lease) < 0) fail(p, "DMA producer queue failed");
    else p->queued++;
    if (!valid) fail(p, "DMA producer layout/write failure");
}
static void tick(void *data, uint64_t expirations)
{
    (void)expirations;
    struct producer *p = data;
    if (pw_stream_trigger_process(p->stream) < 0) fail(p, "DMA producer trigger failed");
}
static void state_changed(void *data, enum pw_stream_state old, enum pw_stream_state state, const char *error)
{
    (void)old;
    struct producer *p = data;
    if (!p->timer) return; // disconnect/destruction after timer release
    if (state == PW_STREAM_STATE_ERROR) fail(p, error ? error : "DMA stream failed");
    else if (state == PW_STREAM_STATE_STREAMING) {
        struct timespec start = { .tv_nsec = 1 }, interval = { .tv_nsec = 40000000 };
        if (pw_loop_update_timer(pw_main_loop_get_loop(p->loop), p->timer, &start, &interval, false) < 0)
            fail(p, "DMA timer start failed");
    } else if (state == PW_STREAM_STATE_PAUSED &&
        pw_loop_update_timer(pw_main_loop_get_loop(p->loop), p->timer, NULL, NULL, false) < 0)
        fail(p, "DMA timer pause failed");
}
static void format_changed(void *data, uint32_t id, const struct spa_pod *param)
{
    struct producer *p = data;
    if (id != SPA_PARAM_Format || !param) return;
    struct spa_video_info_raw format = {0};
    if (spa_format_video_raw_parse(param, &format) < 0 || format.format != SPA_VIDEO_FORMAT_RGB ||
        format.size.width != WIDTH || format.size.height != HEIGHT ||
        !(format.flags & SPA_VIDEO_FLAG_MODIFIER) || format.modifier != 0) {
        fail(p, "DMA producer negotiated an unexpected format/modifier"); return;
    }
    unsigned char storage[512];
    struct spa_pod_builder builder = SPA_POD_BUILDER_INIT(storage, sizeof(storage));
    const struct spa_pod *buffers = spa_pod_builder_add_object(&builder,
        SPA_TYPE_OBJECT_ParamBuffers, SPA_PARAM_Buffers,
        SPA_PARAM_BUFFERS_buffers, SPA_POD_CHOICE_RANGE_Int(8, 2, 16),
        SPA_PARAM_BUFFERS_blocks, SPA_POD_Int(1),
        SPA_PARAM_BUFFERS_size, SPA_POD_Int(4096),
        SPA_PARAM_BUFFERS_stride, SPA_POD_Int(STRIDE),
        SPA_PARAM_BUFFERS_dataType, SPA_POD_CHOICE_FLAGS_Int(1 << SPA_DATA_DmaBuf));
    if (!buffers || pw_stream_update_params(p->stream, &buffers, 1) < 0)
        fail(p, "DMA producer buffer negotiation failed");
}
static void stop(void *data, int signal)
{
    (void)signal;
    struct producer *p = data;
    p->stopping = true;
    pw_main_loop_quit(p->loop);
}
int main(int argc, char **argv)
{
    struct producer p = {0};
    if (argc != 2 || (strcmp(argv[1], "1") != 0 && strcmp(argv[1], "2") != 0)) return 1;
    p.exporter = (uint32_t)(argv[1][0] - '0');
    argc = 1; argv[1] = NULL;
    setvbuf(stdout, NULL, _IOLBF, 0);
    pw_init(&argc, &argv);
    p.loop = pw_main_loop_new(NULL);
    if (!p.loop) { pw_deinit(); return 1; }
    struct pw_context *context = NULL;
    struct pw_core *core = NULL;
    struct pw_loop *loop = pw_main_loop_get_loop(p.loop);
    p.timer = pw_loop_add_timer(loop, tick, &p);
    if (!p.timer || !pw_loop_add_signal(loop, SIGTERM, stop, &p) || !pw_loop_add_signal(loop, SIGINT, stop, &p)) goto failed;
    context = pw_context_new(loop, NULL, 0);
    if (!context) goto failed;
    core = pw_context_connect(context, NULL, 0);
    if (!core) goto failed;
    p.stream = pw_stream_new(core, "mfs-p1-owned-dma-source", pw_properties_new(
        PW_KEY_NODE_NAME, "mfs-p0-fixture", PW_KEY_MEDIA_CLASS, "Video/Source",
        PW_KEY_NODE_SUPPORTS_REQUEST, "1", NULL));
    if (!p.stream) goto failed;
    static const struct pw_stream_events events = {
        PW_VERSION_STREAM_EVENTS, .process = produce, .state_changed = state_changed,
        .param_changed = format_changed, .add_buffer = add_buffer, .remove_buffer = remove_buffer
    };
    pw_stream_add_listener(p.stream, &p.listener, &events, &p);
    unsigned char storage[512];
    struct spa_pod_builder builder = SPA_POD_BUILDER_INIT(storage, sizeof(storage));
    const struct spa_pod *format = spa_pod_builder_add_object(&builder,
        SPA_TYPE_OBJECT_Format, SPA_PARAM_EnumFormat,
        SPA_FORMAT_mediaType, SPA_POD_Id(SPA_MEDIA_TYPE_video),
        SPA_FORMAT_mediaSubtype, SPA_POD_Id(SPA_MEDIA_SUBTYPE_raw),
        SPA_FORMAT_VIDEO_format, SPA_POD_Id(SPA_VIDEO_FORMAT_RGB),
        SPA_FORMAT_VIDEO_size, SPA_POD_Rectangle(&SPA_RECTANGLE(WIDTH, HEIGHT)),
        SPA_FORMAT_VIDEO_framerate, SPA_POD_Fraction(&SPA_FRACTION(25, 1)),
        SPA_FORMAT_VIDEO_modifier, SPA_POD_Long(0));
    if (!format || pw_stream_connect(p.stream, PW_DIRECTION_OUTPUT, PW_ID_ANY,
        PW_STREAM_FLAG_DRIVER | PW_STREAM_FLAG_ALLOC_BUFFERS, &format, 1) < 0) goto failed;
    if (pw_main_loop_run(p.loop) < 0) p.error = 1;
    if (!p.stopping || p.dequeued == 0 || p.dequeued != p.queued || p.writes != p.queued) p.error = 1;
    goto cleanup;
failed:
    p.error = 1;
cleanup:
    if (p.timer) { pw_loop_destroy_source(loop, p.timer); p.timer = NULL; }
    if (p.stream) {
        // remove_buffer must run during disconnect/destruction while p is live.
        if (pw_stream_disconnect(p.stream) < 0) p.error = 1;
        pw_stream_destroy(p.stream);
    }
    // A missing native removal callback remains a failure; reclaim only this
    // fixture's own FDs without counting it as successful native removal.
    for (size_t i = 0; i < 16; i++)
        if (p.slots[i].buffer) { close(p.slots[i].fd); p.error = 1; }
    if (p.allocated == 0 || p.allocated != p.removed) p.error = 1;
    if (core && pw_core_disconnect(core) < 0) p.error = 1;
    if (context) pw_context_destroy(context);
    pw_main_loop_destroy(p.loop);
    pw_deinit();
    printf("owned DMA producer allocated=%" PRIu64 " removed=%" PRIu64
           " dequeued=%" PRIu64 " queued=%" PRIu64 " writes=%" PRIu64 " error=%d\n",
           p.allocated, p.removed, p.dequeued, p.queued, p.writes, p.error);
    return p.error;
}
