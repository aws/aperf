#pragma once

#include "bmiss_map.h"
#include "fname_binary_map.h"
#include "lat_map.h"

bmiss_map_entry_t *deserialize_bmiss_map(const char *filename, uint64_t *out_count);
void generate_bmiss_report(const char *filepath, bmiss_map_entry_t *entries, uint64_t count);

lat_map_entry_t *deserialize_lat_map(const char *filename, uint64_t *out_count);

void generate_lat_report(const char *filepath, lat_map_entry_t *entries, uint64_t count);

int deserialize_maps(int argc, char *argv[]);