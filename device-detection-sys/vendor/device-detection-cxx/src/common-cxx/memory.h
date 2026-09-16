/* *********************************************************************
 * This Original Work is copyright of 51 Degrees Mobile Experts Limited.
 * Copyright 2026 51 Degrees Mobile Experts Limited, Davidson House,
 * Forbury Square, Reading, Berkshire, United Kingdom RG1 3EU.
 *
 * This Original Work is licensed under the European Union Public Licence
 * (EUPL) v.1.2 and is subject to its terms as set out below.
 *
 * If a copy of the EUPL was not distributed with this file, You can obtain
 * one at https://opensource.org/licenses/EUPL-1.2.
 *
 * The 'Compatible Licences' set out in the Appendix to the EUPL (as may be
 * amended by the European Commission) shall be deemed incompatible for
 * the purposes of the Work and the provisions of the compatibility
 * clause in Article 5 of the EUPL shall not apply.
 *
 * If using the Work as, or as part of, a network application, by
 * including the attribution notice(s) required under Article 5 of the EUPL
 * in the end user terms of the application under an appropriate heading,
 * such notice(s) shall fulfill the requirements of that article.
 * ********************************************************************* */

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesMemory Memory
 *
 * Utility methods used to handle common memory operations such as allocating 
 * memory and freeing it, or reading through continuous memory checking for
 * buffer over or under runs.
 *
 * @{
 */

#ifndef FIFTYONE_DEGREES_MEMORY_H_INCLUDED
#define FIFTYONE_DEGREES_MEMORY_H_INCLUDED

#include <stdlib.h>
#include <stdbool.h>
#include "data.h"
#include "threading.h"
#include "tree.h"
#include "common.h"
#include "fileOffset.h"

#ifdef _MSC_VER
#define FIFTYONE_DEGREES_CALL_CONV __cdecl
#else
#define FIFTYONE_DEGREES_CALL_CONV
#endif

/**
 * Used to read data from memory in a similar manner to a file handle.
 */
typedef struct fiftyone_degrees_memory_reader_t {
	byte *startByte; /**< The first byte in memory */
	byte *current; /**< The current byte being read from */
	byte *lastByte; /**< The maximum byte that can be read from */
	fiftyoneDegreesFileOffset length; /**< Length of the file in bytes */
} fiftyoneDegreesMemoryReader;

/**
 * Used to read continuous memory checking for buffer over or under runs.
 * @param reader structure used to check for last byte
 * @param advanceBy number of bytes to advance current by
 * @return true if the advance succeeded, otherwise false
 */
bool fiftyoneDegreesMemoryAdvance(
	fiftyoneDegreesMemoryReader *reader,
	size_t advanceBy);

/**
 * Allocates memory using the standard malloc method.
 * @param __size number of bytes to allocate
 * @return pointer to allocated memory or NULL
 */
EXTERNAL void* fiftyoneDegreesMemoryStandardMalloc(size_t __size);

/**
 * Allocates memory while keeping track of the memory which has been allocated
 * using this method since calling #fiftyoneDegreesMemoryTrackingReset.
 * @param __size number of bytes to allocate
 * @return pointer to allocated memory or NULL
 */
EXTERNAL void* fiftyoneDegreesMemoryTrackingMalloc(size_t __size);

/**
 * Allocated aligned memory using the standard malloc method.
 * @param alignment byte boundary to align the allocation to e.g. 16
 * @param __size number of bytes to allocate
 * @return pointer to allocation memory or NULL
 */
EXTERNAL void* fiftyoneDegreesMemoryStandardMallocAligned(
    int alignment,
    size_t __size);


/**
 * Allocates aligned memory while keeping track of the memory which has been
 * allocated using this method since calling
 * #fiftyoneDegreesMemoryTrackingReset.
 * @param alignment byte boundary to align the allocation to e.g. 16
 * @param __size number of bytes to allocate
 * @return pointer to allocation memory or NULL
 */
EXTERNAL void* fiftyoneDegreesMemoryTrackingMallocAligned(
    int alignment,
    size_t __size);

/**
 * Frees memory allocated using the #fiftyoneDegreesMemoryTrackingMalloc method,
 * noting that it has now been freed and does not contribute to the current
 * total.
 * @param __ptr data to free
 */
EXTERNAL void fiftyoneDegreesMemoryTrackingFree(void *__ptr);

/**
 * Frees memory allocated using the #fiftyoneDegreesMemoryTrackingMallocAligned
 *  method, noting that it has now been freed and does not contribute to the
 * current total.
 * @param __ptr data to free
 */
EXTERNAL void fiftyoneDegreesMemoryTrackingFreeAligned(void* __ptr);

/**
 * Frees memory using the standard free method.
 * @param __ptr data to free
 */
EXTERNAL void fiftyoneDegreesMemoryStandardFree(void *__ptr);

/**
 * Frees memory using the standard aligned free method.
 * @param __ptr data to free
 */
EXTERNAL void fiftyoneDegreesMemoryStandardFreeAligned(void *__ptr);

/**
 * Gets the maximum total number of bytes that have been allocated using the
 * #fiftyoneDegreesMemoryTrackingMalloc method. NOTE: this is the total at any
 * point in time since the #fiftyoneDegreesMemoryTrackingReset method was
 * called, NOT the total allocated in a single allocation.
 * @return maximum total allocation
 */
EXTERNAL size_t fiftyoneDegreesMemoryTrackingGetMax();

/**
 * Gets the current number of bytes allocated using the tracking malloc and free
 * methods.
 * @return total bytes currently allocated
 */
EXTERNAL size_t fiftyoneDegreesMemoryTrackingGetAllocated();

/**
 * Resets the memory trackers keeping track of the data allocated using the
 * #fiftyoneDegreesMemoryTrackingMalloc method. This should always be called
 * before tracking memory allocations.
 */
EXTERNAL void fiftyoneDegreesMemoryTrackingReset();

/**
 * Setup memory tracking by resetting memory tracking and setting all Malloc/Free
 * function pointers to pointer to TrackingMalloc/Free functions.
 */
EXTERNAL void fiftyoneDegreesSetUpMemoryTracking();

/**
 * This function works in collaboration with fiftyoneDegreesSetUpMemoryTracking.
 * Check if all tracking memory has been freed.
 * Unset memory tracking by setting all Malloc/Free function pointer to standard
 * Malloc/Free functions. Then, reset memory tracking.
 * @return 0 if all freed, non 0 if there are memory left unfreed. The returned
 * value is the size of allocated memory left unfreed.
 */
EXTERNAL size_t fiftyoneDegreesUnsetMemoryTracking();

/**
 * Pointer to the method used to allocate memory. By default this maps to
 * #fiftyoneDegreesMemoryStandardMalloc which calls the standard library malloc.
 * @param __size to allocate
 * @return pointer to allocated memory or NULL
 */
EXTERNAL_VAR void *(FIFTYONE_DEGREES_CALL_CONV *fiftyoneDegreesMalloc)(size_t __size);

/**
 * Pointer to the method used to allocate aligned memory. By default this maps
 * to #fiftyoneDegreesMemoryStandardMallocAligned which calls the standard
 * library malloc, allocating slightly more that requested, then ensures the
 * pointer is aligned to a boundary.
 * @param alignment byte boundary to align the allocation to e.g. 16
 * @param __size to allocate
 * @return pointer to allocated memory or NULL
 */
EXTERNAL_VAR void* (FIFTYONE_DEGREES_CALL_CONV *fiftyoneDegreesMallocAligned)(
    int alignment,
    size_t __size);

/**
 * Pointer to the method used to free memory. By default this maps to
 * #fiftyoneDegreesMemoryStandardFree which calls the standard library free.
 * @param __ptr pointer to free
 */
EXTERNAL_VAR void (FIFTYONE_DEGREES_CALL_CONV *fiftyoneDegreesFree)(void *__ptr);

/**
 * Pointer to the method used to free memory. By default this maps to
 * #fiftyoneDegreesMemoryStandardFreeAligned which calls the standard library free.
 * @param __ptr pointer to free
 */
EXTERNAL_VAR void (FIFTYONE_DEGREES_CALL_CONV *fiftyoneDegreesFreeAligned)(void* __ptr);

/**
 * @}
 */

#endif
