/* Only plain values cross between libwebrtc's FFmpeg and the separately
 * built, symbol-prefixed HEVC decoder. Their FFmpeg structs are not ABI compatible. */
#ifndef CIRCORD_VIDEO_HEVC_H
#define CIRCORD_VIDEO_HEVC_H
#include <stdint.h>

struct circord_file_hevc;
struct circord_hevc_frame {
  int width, height, depth;
  unsigned x_shift, y_shift;
  int bt709, full_range;
  const uint8_t *data[3];
  int stride[3];
  int64_t timestamp;
};

struct circord_file_hevc *circord_file_hevc_new(unsigned threads, const uint8_t *extra, int size);
void circord_file_hevc_free(struct circord_file_hevc *decoder);
/* NULL data drains delayed pictures. Times are in the demuxer's stream units. */
int circord_file_hevc_send(struct circord_file_hevc *decoder, const uint8_t *data, int size,
                           int64_t pts, int64_t dts, int64_t duration);
/* 1 for a picture, 0 for more input or end, negative for an error.
 * Planes remain valid until the next receive or flush. */
int circord_file_hevc_receive(struct circord_file_hevc *decoder, struct circord_hevc_frame *frame);
void circord_file_hevc_flush(struct circord_file_hevc *decoder);
#endif
