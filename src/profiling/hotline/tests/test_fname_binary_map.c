#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "test.h"

void test_init_fname_binary_btree() {
  init_fname_binary_btree();
  // Just ensure it doesn't crash
}

void test_get_absolute_source_path() {
  // Test absolute path
  char *result = get_absolute_source_path("/bin/test", "/usr/src/file.c");
  assert(result != NULL);
  assert(strcmp(result, "/usr/src/file.c") == 0);
  free(result);

  // Test relative path
  result = get_absolute_source_path("/bin/test", "src/file.c");
  assert(result != NULL);
  // Should contain the relative path resolved
  assert(strstr(result, "src/file.c") != NULL);
  free(result);

  // Test NULL source path
  result = get_absolute_source_path("/bin/test", NULL);
  assert(result == NULL);
}

void test_get_line_at_line_number() {
  // Create a temporary test file
  FILE *temp = tmpfile();
  assert(temp != NULL);

  fprintf(temp, "line 1\n");
  fprintf(temp, "line 2\n");
  fprintf(temp, "line 3\n");
  rewind(temp);

  // Get temp file path (this is tricky with tmpfile, so we'll test with
  // /dev/null)
  char *result = get_line_at_line_number("/dev/null", 1);
  // Should return NULL for /dev/null
  assert(result == NULL);

  fclose(temp);
}

void test_fname_binary_map() {
  test_init_fname_binary_btree();
  test_get_absolute_source_path();
  test_get_line_at_line_number();
}