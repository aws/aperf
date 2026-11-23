#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

void test_bmiss_map();
void test_lat_map();
void test_fname_map();
void test_finode_map();
void test_fname_binary_map();
void test_config();

void test_all() {
  test_bmiss_map();
  test_lat_map();
  test_finode_map();
  test_fname_binary_map();
  test_config();
}
