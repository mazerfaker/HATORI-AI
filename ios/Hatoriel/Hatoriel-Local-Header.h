#ifndef HATORIEL_LOCAL_H
#define HATORIEL_LOCAL_H
#include <stdint.h>
#include <stddef.h>
typedef int (*hatoriel_local_callback)(const uint8_t*,size_t,void*);
int hatoriel_local_generate(const char*,const char* const*,const char* const*,size_t,int32_t,int32_t,hatoriel_local_callback,void*,char*,size_t);
#endif
