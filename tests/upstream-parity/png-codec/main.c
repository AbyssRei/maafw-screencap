// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include <zlib.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef MFS_ZLIB_ONLY
int main(int argc,char** argv) {
    if(argc!=2)return 1;
    if(strcmp(argv[1],ZLIB_VERSION) || strcmp(argv[1],zlibVersion())){
        fprintf(stderr,"ZLIB codec version mismatch: expected=%s header=%s runtime=%s\n",argv[1],ZLIB_VERSION,zlibVersion());return 2;
    }
    const unsigned char pixels[24]={1,7,19,31,43,59,71,83,97,109,127,139,151,163,179,191,211,223,239,251,3,13,23,37};
    unsigned char encoded[128]={0},decoded[24]={0};
    uLongf encoded_size=sizeof(encoded),decoded_size=sizeof(decoded);
    if(compressBound(sizeof(pixels))>sizeof(encoded))return 3;
    if(compress2(encoded,&encoded_size,pixels,sizeof(pixels),Z_BEST_COMPRESSION)!=Z_OK)return 4;
    if(uncompress(decoded,&decoded_size,encoded,encoded_size)!=Z_OK)return 5;
    if(decoded_size!=sizeof(pixels) || memcmp(decoded,pixels,sizeof(pixels)))return 6;
    printf("ZLIB codec zlib=%s actual_compress_decompress=1 known_bytes=24 passed\n",zlibVersion());
    return 0;
}
#else
#include <png.h>
int main(int argc,char** argv) {
    if(argc!=3)return 1;
    if(strcmp(argv[1],PNG_LIBPNG_VER_STRING) || strcmp(argv[1],png_get_libpng_ver(NULL))
       || strcmp(argv[2],ZLIB_VERSION) || strcmp(argv[2],zlibVersion())){
        fprintf(stderr,"PNG codec version mismatch: expected png=%s zlib=%s, header png=%s zlib=%s, runtime png=%s zlib=%s\n",
            argv[1],argv[2],PNG_LIBPNG_VER_STRING,ZLIB_VERSION,png_get_libpng_ver(NULL),zlibVersion());return 2;
    }
    const unsigned char pixels[24]={1,7,19,31,43,59,71,83,97,109,127,139,151,163,179,191,211,223,239,251,3,13,23,37};
    png_image writer={0};writer.version=PNG_IMAGE_VERSION;writer.width=3;writer.height=2;writer.format=PNG_FORMAT_RGBA;
    png_alloc_size_t size=0;
    if(!png_image_write_to_memory(&writer,NULL,&size,0,pixels,0,NULL))return 3;
    void* encoded=malloc(size);
    if(!encoded)return 4;
    int status=0;
    png_image reader={0};reader.version=PNG_IMAGE_VERSION;
    if(!png_image_write_to_memory(&writer,encoded,&size,0,pixels,0,NULL)){status=5;goto done;}
    if(!png_image_begin_read_from_memory(&reader,encoded,size)){status=6;goto done;}
    if(reader.width!=3 || reader.height!=2){status=7;goto done;}
    reader.format=PNG_FORMAT_RGBA;
    unsigned char decoded[24]={0};
    if(!png_image_finish_read(&reader,NULL,decoded,0,NULL)){status=8;goto done;}
    if(memcmp(decoded,pixels,sizeof(pixels))){status=9;goto done;}
    printf("PNG codec png=%s zlib=%s actual_encode_decode=1 known_rgba_pixels=6 passed\n",png_get_libpng_ver(NULL),zlibVersion());
done:
    if(status)fprintf(stderr,"PNG codec status=%d writer=%s reader=%s\n",status,writer.message,reader.message);
    png_image_free(&reader);png_image_free(&writer);free(encoded);return status;
}
#endif
