#ifndef HATORIEL_BRIDGE_H
#define HATORIEL_BRIDGE_H
#include <stdint.h>
#include <stddef.h>
#include "Hatoriel-Local-Header.h"
char *hatoriel_version(void);
char *hatoriel_prepare(const uint8_t *data, size_t len);
char *hatoriel_prepare_context(const uint8_t *data, size_t len);
void hatoriel_free(char *value);
#endif
