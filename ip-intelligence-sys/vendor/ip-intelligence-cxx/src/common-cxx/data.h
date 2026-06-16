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

#ifndef FIFTYONE_DEGREES_DATA_H_INCLUDED
#define FIFTYONE_DEGREES_DATA_H_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesData Data
 *
 * Structure containing memory allocated to store a variable.
 *
 * ## Terms
 *
 * **Data** : a structure containing memory allocated to a certain item. This
 * can be of any type.
 *
 * ## Introduction
 *
 * A Data structure contains memory allocated for the purpose of storing a
 * variable. This differs from a direct pointer in that the memory can be
 * reused for another purpose. By keeping track of how much data has been
 * allocated, the same allocation can be reused if memory of equal or smaller
 * size is needed, otherwise more memory can be automatically allocated in the
 * same method call.
 *
 * ## Creation
 * 
 * A Data structure does not need to be created by a method, it only needs to
 * be allocated in memory. However, it does need to be reset before any
 * operations are carried out with it.
 *
 * ## Allocation
 *
 * To allocate memory for a variable in a Data structure, the 
 * #fiftyoneDegreesDataMalloc method can be called in a similar way to the
 * standard malloc method. Any existing variable in the Data structure will
 * either be overwritten by the new variable, or the previous variable will be
 * freed and new memory allocated.
 *
 * ## Example Usage
 *
 * ```
 * // Allocate a data structure
 * fiftyoneDegreesData *data = Malloc(sizeof(fiftyoneDegreesData));
 *
 * // Reset the data ready to allocate something
 * fiftyoneDegreesDataReset(data);
 *
 * // Create the data to store
 * const char *string = "some data";
 * size_t size = strlen(string) * sizeof(char);
 *
 * // Allocate the memory inside the data structure
 * void *dataPtr = fiftyoneDegreesDataMalloc(data, size);
 *
 * // Check the allocation was succesful
 * if (dataPtr != NULL) {
 *
 *     // Set the data in the data structure and size it uses
 *     strcpy((char*)dataPtr, string);
 *     data->used = size;
 * }
 * ```
 *
 * @{
 */

#include <stdint.h>
#include <stddef.h>
#include "common.h"

/**
 * Alias for unsigned char
 */
typedef unsigned char byte;

/**
 * Data structure used for reusing memory which may have been allocated in a
 * previous operation.
 */
typedef struct fiftyone_degrees_data_t {
	byte *ptr; /**< Pointer to immutable data */
	uint32_t allocated; /**< Number of bytes allocated at the pointer. Used 
	                        within the collection and is not intended for
	                        consumers to reference. */
	uint32_t used; /**< Number of valid bytes currently referenced by pointer */
} fiftyoneDegreesData;

/**
 * Resets the data structure ready for a new operation. **MUST** be called
 * before using an instance of #fiftyoneDegreesData.
 * @param data to be reset
 */
EXTERNAL void fiftyoneDegreesDataReset(fiftyoneDegreesData *data);

/**
 * Ensures the data structure contains sufficient bytes. If insufficient bytes
 * are available then the current memory is freed and a new block of memory is 
 * allocated.
 * @param data pointer to the data structure to be checked for sufficient bytes
 * @param bytesNeeded the number of bytes the data needs to be able to store
 * @return a pointer to the memory held within data
 */
EXTERNAL void* fiftyoneDegreesDataMalloc(
	fiftyoneDegreesData *data,
	size_t bytesNeeded);

/**
 * @}
 */
#endif
