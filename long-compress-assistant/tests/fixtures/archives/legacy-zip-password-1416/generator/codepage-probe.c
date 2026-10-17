#include <windows.h>
#include <stdio.h>
int main(int argc, char **argv) {
  printf("ACP=%u OEMCP=%u PASSWORD_HEX=", GetACP(), GetOEMCP());
  if (argc > 1) for (const unsigned char *p = (const unsigned char *)argv[1]; *p; ++p) printf("%02x", *p);
  puts("");
  return 0;
}
