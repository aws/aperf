#include "report.h"

#include <string.h>

#include "btree.h"
#include "config.h"
#include "fname_binary_map.h"
#include "log.h"
#include "sys.h"

#define MIN(a, b) ((a) < (b) ? (a) : (b))

/// @brief Reads the bmiss_map serialized file and stores it in memory
/// @param filename Filename to read in
/// @param out_count Count to populate for later iteration
/// @return Pointer to malloc-ed file struct array
bmiss_map_entry_t *deserialize_bmiss_map(const char *filename, uint64_t *out_count) {
  FILE *fp = fopen(filename, "r");
  ASSERT(fp != NULL, "Failed to open binary file.");

  char *line = NULL;
  size_t len = 0;
  ssize_t read;
  size_t entry_count = 0;

  size_t capacity = 64;
  bmiss_map_entry_t *entries = malloc(capacity * sizeof(bmiss_map_entry_t));
  ASSERT(entries != NULL, "Failed to allocate memory for entries.");

  if ((read = getline(&line, &len, fp)) == -1) {
    free(line);
    free(entries);
    fclose(fp);
    return NULL;
  }

  while ((read = getline(&line, &len, fp)) != -1) {
    if (entry_count >= capacity) {
      capacity *= 2;
      bmiss_map_entry_t *temp = realloc(entries, capacity * sizeof(bmiss_map_entry_t));
      if (temp == NULL) {
        free(entries);
        free(line);
        fclose(fp);
        ASSERT(0, "Failed to reallocate memory for entries.");
        return NULL;
      }
      entries = temp;
    }

    bmiss_map_entry_t *entry = &entries[entry_count];

    // Temporary buffer for filename
    char temp_filename[4096] = {0};

    int result = sscanf(line, "%[^,],%lx,%lu,%lu,%hhx", temp_filename, &entry->offset,
                        &entry->count, &entry->mispredicted, &entry->branch_type);

    ASSERT(result == 5, "failed to sscanf bmiss line correctly.");

    // Allocate memory for the filename and copy it
    entry->filename = strdup(temp_filename);
    if (entry->filename == NULL) {
      // Handle memory allocation failure
      fprintf(stderr, "Failed to allocate memory for filename\n");
      continue;
    }
    entry_count++;
  }

  if (entry_count > 0) {
    bmiss_map_entry_t *temp = realloc(entries, entry_count * sizeof(bmiss_map_entry_t));
    if (temp != NULL) {
      entries = temp;
    }
  }

  *out_count = entry_count;
  free(line);
  fclose(fp);
  return entries;
}

/// @brief Reads the lat_map serialized file and stores it in memory
/// @param filename Filename to read in
/// @param out_count Count to populate for later iteration
/// @return Pointer to malloc-ed file struct array
lat_map_entry_t *deserialize_lat_map(const char *filename, uint64_t *out_count) {
  FILE *fp = fopen(filename, "r");
  ASSERT(fp != NULL, "Failed to open binary file.");

  char *line = NULL;
  size_t len = 0;
  ssize_t read;
  size_t entry_count = 0;

  size_t capacity = 64;
  lat_map_entry_t *entries = malloc(capacity * sizeof(lat_map_entry_t));
  ASSERT(entries != NULL, "Failed to allocate memory for entries.");

  if ((read = getline(&line, &len, fp)) == -1) {
    free(line);
    free(entries);
    fclose(fp);
    return NULL;
  }

  while ((read = getline(&line, &len, fp)) != -1) {
    if (entry_count >= capacity) {
      capacity *= 2;
      lat_map_entry_t *temp = realloc(entries, capacity * sizeof(lat_map_entry_t));
      if (temp == NULL) {
        free(entries);
        free(line);
        fclose(fp);
        ASSERT(0, "Failed to reallocate memory for entries.");
        return NULL;
      }
      entries = temp;
    }

    lat_map_entry_t *entry = &entries[entry_count];

    // Temporary buffer for filename
    char temp_filename[4096];

    int result = sscanf(line,
                        "%[^,],%lx,%ld,%ld,%ld,%ld,%ld,%ld,%ld,%ld,%ld,%ld,%ld,"
                        "%ld,%ld,%ld,%ld,%ld,%ld,%ld,%ld,%ld,"
                        "%ld",
                        temp_filename, &entry->offset, &entry->count, &entry->total_latency,
                        &entry->issue_latency, &entry->translation_latency, &entry->l1.l1_bound_bin,
                        &entry->l1.l2_bound_bin, &entry->l1.l3_bound_bin, &entry->l1.dram_bound_bin,
                        &entry->l2.l1_bound_bin, &entry->l2.l2_bound_bin, &entry->l2.l3_bound_bin,
                        &entry->l2.dram_bound_bin, &entry->l3.l1_bound_bin, &entry->l3.l2_bound_bin,
                        &entry->l3.l3_bound_bin, &entry->l3.dram_bound_bin,
                        &entry->dram.l1_bound_bin, &entry->dram.l2_bound_bin,
                        &entry->dram.l3_bound_bin, &entry->dram.dram_bound_bin, &entry->saturated);

    ASSERT(result == 23, "failed to sscanf lat line correctly.");
    // Allocate memory for the filename and copy it
    entry->filename = strdup(temp_filename);
    ASSERT(entry->filename != NULL, "Failed to copy filename.");
    entry_count++;
  }

  if (entry_count > 0) {
    lat_map_entry_t *temp = realloc(entries, entry_count * sizeof(lat_map_entry_t));
    if (temp != NULL) {
      entries = temp;
    }
  }

  *out_count = entry_count;
  free(line);
  fclose(fp);
  return entries;
}

/// @brief Helper function to set up a file pointer for the reports
/// @param file_dir Directory to create file
/// @param filename Name of file
/// @return File pointer to file
FILE *setup_report_file(const char *file_dir, char *filename) {
  size_t path_len = strlen(file_dir) + strlen(filename) + 2;
  char *filepath = malloc(path_len);
  ASSERT(filepath != NULL, "Failed to malloc file path.");
  snprintf(filepath, path_len, "%s/%s", file_dir, filename);
  FILE *fp = fopen(filepath, "w");
  ASSERT(fp != NULL, "Failed to open file for writing.");

  return fp;
}

/// @brief Used for sorting bmiss entries
/// @param a First entry to compare
/// @param b Second entry to compare
/// @return 1 if a > b, -1 if a < b, 0 if a = b
int compare_bmiss_entries(const void *a, const void *b) {
  const bmiss_map_entry_t *entry_a = (const bmiss_map_entry_t *)a;
  const bmiss_map_entry_t *entry_b = (const bmiss_map_entry_t *)b;

  // Sort by count (descending)
  if (entry_b->count > entry_a->count) return 1;
  if (entry_b->count < entry_a->count) return -1;
  return 0;
}

/// @brief Generates the bmiss report after process_bmiss_entries is called
/// @param file_dir File dir to generate report in
/// @param entries bmiss_map_t entries to populate report with
/// @param count Number of entries
void generate_bmiss_report(const char *file_dir, bmiss_map_entry_t *entries, uint64_t count) {
  FILE *fp = setup_report_file(file_dir, "hotline_bmiss_map.csv");

  // write the header in
  fprintf(fp,
          "Sample Count,Mispredicted (%%),"
          "Location:Line,Source Line,Function,Assembly,Type\n");

  // Don't want to crash in case other reports have entries
  if (entries == NULL || count == 0) {
    fprintf(stderr, "No entries to process\n");
    fclose(fp);
    return;
  }

  qsort(entries, count, sizeof(bmiss_map_entry_t), compare_bmiss_entries);
  for (size_t i = 0; i < MIN(count, profile_configuration.num_to_report); i++) {
    const bmiss_map_entry_t *entry = &entries[i];

    debug_info_t *dinfo = get_debug_info(entry->filename, entry->offset);

    fprintf(
        fp, "%ld,%.2f%%,%s:%ld,%s,%s,%s,%s\n", entries[i].count,
        (entries[i].count > 0) ? ((double)entries[i].mispredicted / entries[i].count) * 100.0 : 0.0,
        dinfo->src_file, dinfo->line_num, dinfo->line, dinfo->function, dinfo->assembly,
        entries[i].branch_type == 0x01 ? "Conditional" : "Indirect");
  }

  fclose(fp);
}

/// @brief Comparison function for lat_map exec latencies
/// @param a First entry to compare
/// @param b Second entry to compre
/// @return 1 if a > b, -1 if a < b, 0 if a = b
int compare_lat_exec_entries(const void *a, const void *b) {
  const lat_map_entry_t *entry_a = (const lat_map_entry_t *)a;
  const lat_map_entry_t *entry_b = (const lat_map_entry_t *)b;

  // Calculate execution latencies
  long exec_a = entry_a->total_latency - entry_a->issue_latency - entry_a->translation_latency;
  long exec_b = entry_b->total_latency - entry_b->issue_latency - entry_b->translation_latency;

  // Sort by execution latency (descending)
  if (exec_b > exec_a) return 1;
  if (exec_b < exec_a) return -1;
  return 0;
}

/// @brief Writes a single exec latency entry to the file
/// @param fp File pointer of file to write to
/// @param entry Pointer to entry information
/// @param dinfo Debug information struct
void print_exec_latency(FILE *fp, const lat_map_entry_t *entry, const debug_info_t *dinfo) {
  fprintf(fp, "%.2f,%ld,%ld,%s:%ld,%s,%s,%s\n",
          (double)(entry->total_latency - entry->issue_latency - entry->translation_latency) /
              (entry->count * 1000.0),
          entry->count, entry->saturated, dinfo->src_file, dinfo->line_num, dinfo->line,
          dinfo->function, dinfo->assembly);
}

/// @brief Comparison function for lat_map issue latencies
/// @param a First entry to compare
/// @param b Second entry to compre
/// @return 1 if a > b, -1 if a < b, 0 if a = b
int compare_lat_issue_entries(const void *a, const void *b) {
  const lat_map_entry_t *entry_a = (const lat_map_entry_t *)a;
  const lat_map_entry_t *entry_b = (const lat_map_entry_t *)b;

  // Sort by issue latency (descending)
  if (entry_b->issue_latency > entry_a->issue_latency) return 1;
  if (entry_b->issue_latency < entry_a->issue_latency) return -1;
  return 0;
}

/// @brief Writes a single issue latency entry to the file
/// @param fp File pointer of file to write to
/// @param entry Pointer to entry information
/// @param dinfo Debug information struct
void print_issue_latency(FILE *fp, const lat_map_entry_t *entry, const debug_info_t *dinfo) {
  fprintf(fp, "%.2f,%ld,%ld,%s:%ld,%s,%s,%s\n",
          (double)(entry->issue_latency) / (entry->count * 1000.0), entry->count, entry->saturated,
          dinfo->src_file, dinfo->line_num, dinfo->line, dinfo->function, dinfo->assembly);
}

/// @brief Comparison function for lat_map translation latencies
/// @param a First entry to compare
/// @param b Second entry to compre
/// @return 1 if a > b, -1 if a < b, 0 if a = b
int compare_translation_issue_entries(const void *a, const void *b) {
  const lat_map_entry_t *entry_a = (const lat_map_entry_t *)a;
  const lat_map_entry_t *entry_b = (const lat_map_entry_t *)b;

  // Sort by translation latency (descending)
  if (entry_b->translation_latency > entry_a->translation_latency) return 1;
  if (entry_b->translation_latency < entry_a->translation_latency) return -1;
  return 0;
}

/// @brief Writes a single translation latency entry to the file
/// @param fp File pointer of file to write to
/// @param entry Pointer to entry information
/// @param dinfo Debug information struct
void print_translation_latency(FILE *fp, const lat_map_entry_t *entry, const debug_info_t *dinfo) {
  fprintf(fp, "%.2f,%ld,%ld,%s:%ld,%s,%s,%s\n",
          (double)(entry->translation_latency) / (entry->count * 1000.0), entry->count,
          entry->saturated, dinfo->src_file, dinfo->line_num, dinfo->line, dinfo->function,
          dinfo->assembly);
}

/// @brief Comparison function for lat_map completion node latencies
/// @param a First entry to compare
/// @param b Second entry to compre
/// @return 1 if a > b, -1 if a < b, 0 if a = b
int compare_completion_node_issue_entries(const void *a, const void *b) {
  const lat_map_entry_t *entry_a = (const lat_map_entry_t *)a;
  const lat_map_entry_t *entry_b = (const lat_map_entry_t *)b;

  // Sort by total latency (descending)
  if (entry_b->total_latency > entry_a->total_latency) return 1;
  if (entry_b->total_latency < entry_a->total_latency) return -1;
  return 0;
}

/// @brief Writes a single completion node entry to the file
/// @param fp File pointer of file to write to
/// @param entry Pointer to entry information
/// @param dinfo Debug information struct
void print_completion_node(FILE *fp, const lat_map_entry_t *entry, const debug_info_t *dinfo) {
  // Calculate totals for each level
  uint64_t l1_total = entry->l1.l1_bound_bin + entry->l1.l2_bound_bin + entry->l1.l3_bound_bin +
                      entry->l1.dram_bound_bin;
  uint64_t l2_total = entry->l2.l1_bound_bin + entry->l2.l2_bound_bin + entry->l2.l3_bound_bin +
                      entry->l2.dram_bound_bin;
  uint64_t l3_total = entry->l3.l1_bound_bin + entry->l3.l2_bound_bin + entry->l3.l3_bound_bin +
                      entry->l3.dram_bound_bin;
  uint64_t dram_total = entry->dram.l1_bound_bin + entry->dram.l2_bound_bin +
                        entry->dram.l3_bound_bin + entry->dram.dram_bound_bin;

  uint64_t grand_total = l1_total + l2_total + l3_total + dram_total;
  if (grand_total == 0) grand_total = 1;

  // Calculate level percentages
  double l1_pct = (double)l1_total / grand_total * 100.0;
  double l2_pct = (double)l2_total / grand_total * 100.0;
  double l3_pct = (double)l3_total / grand_total * 100.0;
  double dram_pct = (double)dram_total / grand_total * 100.0;

  // Calculate bin percentages for L1
  double l1_bins[4] = {l1_total > 0 ? (double)entry->l1.l1_bound_bin / l1_total * 100.0 : 0.0,
                       l1_total > 0 ? (double)entry->l1.l2_bound_bin / l1_total * 100.0 : 0.0,
                       l1_total > 0 ? (double)entry->l1.l3_bound_bin / l1_total * 100.0 : 0.0,
                       l1_total > 0 ? (double)entry->l1.dram_bound_bin / l1_total * 100.0 : 0.0};

  // Calculate bin percentages for L2
  double l2_bins[4] = {l2_total > 0 ? (double)entry->l2.l1_bound_bin / l2_total * 100.0 : 0.0,
                       l2_total > 0 ? (double)entry->l2.l2_bound_bin / l2_total * 100.0 : 0.0,
                       l2_total > 0 ? (double)entry->l2.l3_bound_bin / l2_total * 100.0 : 0.0,
                       l2_total > 0 ? (double)entry->l2.dram_bound_bin / l2_total * 100.0 : 0.0};

  // Calculate bin percentages for L3
  double l3_bins[4] = {l3_total > 0 ? (double)entry->l3.l1_bound_bin / l3_total * 100.0 : 0.0,
                       l3_total > 0 ? (double)entry->l3.l2_bound_bin / l3_total * 100.0 : 0.0,
                       l3_total > 0 ? (double)entry->l3.l3_bound_bin / l3_total * 100.0 : 0.0,
                       l3_total > 0 ? (double)entry->l3.dram_bound_bin / l3_total * 100.0 : 0.0};

  // Calculate bin percentages for DRAM
  double dram_bins[4] = {
      dram_total > 0 ? (double)entry->dram.l1_bound_bin / dram_total * 100.0 : 0.0,
      dram_total > 0 ? (double)entry->dram.l2_bound_bin / dram_total * 100.0 : 0.0,
      dram_total > 0 ? (double)entry->dram.l3_bound_bin / dram_total * 100.0 : 0.0,
      dram_total > 0 ? (double)entry->dram.dram_bound_bin / dram_total * 100.0 : 0.0};

  fprintf(fp,
          "%.3f,%.3f | %.3f | %.3f | %.3f,"
          "%.3f,%.3f | %.3f | %.3f | %.3f,"
          "%.3f,%.3f | %.3f | %.3f | %.3f,"
          "%.3f,%.3f | %.3f | %.3f | %.3f,"
          "%s:%ld,%s,%s,%s\n",
          l1_pct, l1_bins[0], l1_bins[1], l1_bins[2], l1_bins[3], l2_pct, l2_bins[0], l2_bins[1],
          l2_bins[2], l2_bins[3], l3_pct, l3_bins[0], l3_bins[1], l3_bins[2], l3_bins[3], dram_pct,
          dram_bins[0], dram_bins[1], dram_bins[2], dram_bins[3], dinfo->src_file, dinfo->line_num,
          dinfo->line, dinfo->function, dinfo->assembly);
}

/// @brief Parametrized function to write a sub-report for lat_map
/// @param entries Entries to write
/// @param count Number of entries
/// @param fp File pointer of file to write into
/// @param compare_fn Comparison function for sorting entries
/// @param print_fn Print function to write a single entry into `fp`
void write_lat_map_sub_report(lat_map_entry_t *entries, uint64_t count, FILE *fp,
                              int (*compare_fn)(const void *, const void *),
                              void (*print_fn)(FILE *, const lat_map_entry_t *,
                                               const debug_info_t *)) {
  qsort(entries, count, sizeof(lat_map_entry_t), compare_fn);

  for (size_t i = 0; i < MIN(count, profile_configuration.num_to_report); i++) {
    const lat_map_entry_t *entry = &entries[i];
    debug_info_t *dinfo = get_debug_info(entry->filename, entry->offset);
    print_fn(fp, entry, dinfo);
  }
}

/// @brief Calls write_lat_map_sub_report for each sub-view
/// @param file_dir File directory to put reports into
/// @param entries Entries to write
/// @param count Number of entries
void generate_lat_report(const char *file_dir, lat_map_entry_t *entries, uint64_t count) {
  FILE *exec_fp = setup_report_file(file_dir, "hotline_lat_map_exec_report.csv");
  FILE *issue_fp = setup_report_file(file_dir, "hotline_lat_map_issue_report.csv");
  FILE *translation_fp = setup_report_file(file_dir, "hotline_lat_map_translation_report.csv");
  FILE *completion_fp = setup_report_file(file_dir, "hotline_lat_map_completion_report.csv");

  // write the headers in
  fputs(
      "Latency (ns),Sample Count,Dropped Packets,Location:Line,"
      "Source Line,Function,Assembly\n",
      exec_fp);

  fputs(
      "Latency (ns),Sample Count,Dropped Packets,Location:Line,"
      "Source Line,Function,Assembly\n",
      issue_fp);

  fputs(
      "Latency (ns),Sample Count,Dropped Packets,Location:Line,"
      "Source Line,Function,Assembly\n",
      translation_fp);

  fprintf(completion_fp,
          "L1 (%%),L1 latencies (%% <= %.1fns | %% <= %.1fns | %% <= %.1fns | %% > %.1fns),"
          "L2 (%%),L2 latencies (%% <= %.1fns | %% <= %.1fns | %% <= %.1fns | %% > %.1fns),"
          "L3 (%%),L3 latencies (%% <= %.1fns | %% <= %.1fns | %% <= %.1fns | %% > %.1fns),"
          "DRAM (%%),DRAM latencies (%% <= %.1fns | %% <= %.1fns | %% <= %.1fns | %% > %.1fns),"
          "Location:Line,Source Line,Function,Assembly\n",
          cpu_system_config.latency_limits.l1_latency_cap_ps / 1000.0,
          cpu_system_config.latency_limits.l2_latency_cap_ps / 1000.0,
          cpu_system_config.latency_limits.l3_latency_cap_ps / 1000.0,
          cpu_system_config.latency_limits.l3_latency_cap_ps / 1000.0,
          cpu_system_config.latency_limits.l1_latency_cap_ps / 1000.0,
          cpu_system_config.latency_limits.l2_latency_cap_ps / 1000.0,
          cpu_system_config.latency_limits.l3_latency_cap_ps / 1000.0,
          cpu_system_config.latency_limits.l3_latency_cap_ps / 1000.0,
          cpu_system_config.latency_limits.l1_latency_cap_ps / 1000.0,
          cpu_system_config.latency_limits.l2_latency_cap_ps / 1000.0,
          cpu_system_config.latency_limits.l3_latency_cap_ps / 1000.0,
          cpu_system_config.latency_limits.l3_latency_cap_ps / 1000.0,
          cpu_system_config.latency_limits.l1_latency_cap_ps / 1000.0,
          cpu_system_config.latency_limits.l2_latency_cap_ps / 1000.0,
          cpu_system_config.latency_limits.l3_latency_cap_ps / 1000.0,
          cpu_system_config.latency_limits.l3_latency_cap_ps / 1000.0);

  // Don't want to crash in case other reports have entries
  if (entries == NULL || count == 0) {
    fprintf(stderr, "No entries to process\n");
    fclose(exec_fp);
    fclose(issue_fp);
    fclose(translation_fp);
    fclose(completion_fp);
    return;
  }

  write_lat_map_sub_report(entries, count, exec_fp, compare_lat_exec_entries, print_exec_latency);
  write_lat_map_sub_report(entries, count, issue_fp, compare_lat_issue_entries,
                           print_issue_latency);
  write_lat_map_sub_report(entries, count, translation_fp, compare_translation_issue_entries,
                           print_translation_latency);
  write_lat_map_sub_report(entries, count, completion_fp, compare_completion_node_issue_entries,
                           print_completion_node);

  fclose(exec_fp);
  fclose(issue_fp);
  fclose(translation_fp);
  fclose(completion_fp);
}

/// @brief Wrapper exposed for APerf to call. Complementary to
/// hotline.c/serialize_maps. Expands the maps and builds individual views.
/// @param argc C-like number of arguments
/// @param argv C-like pointer to argument
/// @return 0 on success, -1 otherwise
int deserialize_maps(int argc, char *argv[]) {
  init_fname_binary_btree();
  init_sys_info();  // this will give us access to our latency metrics
  parse_arguments(argc, argv);

  uint64_t count;

  // For bmiss map data
  // For bmiss map data
  size_t bmiss_len = strlen(profile_configuration.data_dir) +
                     strlen(profile_configuration.bmiss_map_filename) + 2;  // +2 for '/' and '\0'

  char *bmiss_data_path = malloc(bmiss_len);
  if (!bmiss_data_path) {
    perror("Failed to allocate memory for bmiss path");
    return -1;
  }

  int res = snprintf(bmiss_data_path, bmiss_len, "%s/%s", profile_configuration.data_dir,
                     profile_configuration.bmiss_map_filename);

  ASSERT(res > 0, "snprintf failed.");

  // Process bmiss entries
  bmiss_map_entry_t *b_entries = deserialize_bmiss_map(bmiss_data_path, &count);
  generate_bmiss_report(profile_configuration.data_dir, b_entries, count);
  free(bmiss_data_path);

  // For lat map data
  size_t lat_len =
      strlen(profile_configuration.data_dir) + strlen(profile_configuration.lat_map_filename) + 2;
  char *lat_data_path = malloc(lat_len);
  if (!lat_data_path) {
    perror("Failed to allocate memory for lat path");
    return -1;
  }
  res = snprintf(lat_data_path, lat_len, "%s/%s", profile_configuration.data_dir,
                 profile_configuration.lat_map_filename);
  ASSERT(res > 0, "snprintf failed.");

  // Process lat entries
  lat_map_entry_t *l_entries = deserialize_lat_map(lat_data_path, &count);
  generate_lat_report(profile_configuration.data_dir, l_entries, count);
  free(lat_data_path);
  return 0;
}
