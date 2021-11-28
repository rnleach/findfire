
#include "satfire.h"

#include <assert.h>

#include "gdal.h"

void
satfire_initialize(void)
{
    GDALAllRegister();
}

void
satfire_finalize(void)
{
}
