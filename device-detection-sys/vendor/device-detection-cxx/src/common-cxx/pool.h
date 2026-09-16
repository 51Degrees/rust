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

#ifndef FIFTYONE_DEGREES_POOL_H_INCLUDED
#define FIFTYONE_DEGREES_POOL_H_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesPool Pool
 *
 * Pool of handles to allow safe access to multiple threads.
 *
 * ## Introduction
 *
 * A Pool structure defines a pool of handles to a resource, and allows a safe
 * way to access them in a multi-threaded environment.
 *
 * ## Creation
 *
 * The #fiftyoneDegreesPoolInit method is used to initialise a pointer to
 * a #fiftyoneDegreesPool. A concurrency value is provided to indicate the
 * maximum number of threads that will be in operation. If this value is lower
 * than the actual number of threads the stack can be exhausted and a null
 * pointer is returned instead of a valid item. The concurrency value must
 * always be the same or greater than the number of threads. When compiled
 * in single threaded operation a pool is not strictly required and the
 * implementation maintains a simple stack for consistency of interface and to
 * minimise divergent code.
 *
 * ## Get & Release
 *
 * Handles are retrieved from the pool via the #fiftyoneDegreesPoolItemGet
 * method. The handle **MUST** be returned with the
 * #fiftyoneDegreesPoolItemRelease method when it is finished with. The
 * handle will always be open and ready for read only operation. If too many
 * threads are accessing the pool simultaneously, meaning a handle cannot be
 * secured, then a NULL pointer is returned.
 *
 * ## Free
 *
 * The items are closed when the pool is released via the
 *  #fiftyoneDegreesPoolFree method. Any memory allocated by the implementation
 * for the stack is freed.
 *
 * ## Usage Example
 *
 * ```
 * FIFTYONE_DEGREES_EXCEPTION_CREATE;
 * void *state;
 * fiftyoneDegreesPoolResourceCreate resourceCreate;
 * fiftyoneDegreesPoolResourceFree resourceFree;
 * fiftyoneDegreesPool pool;
 *
 * // Initialise a pool of resources and check that it was successful
 * if (fiftyoneDegreesPoolInit(
 *     &pool,
 *     1,
 *     state,
 *     resourceCreate,
 *     resourceFree,
 *     exception) != NULL) {
 *
 *     // Get a resource from the pool
 *     fiftyoneDegreesPoolItem *item = fiftyoneDegreesPoolItemGet(
 *         &pool,
 *         exception);
 *
 *     // Check that there was a resource available
 *     if (item != NULL) {
 *
 *         // Do something with the resource without being interrupted by other
 *         // threads
 *         // ...
 *
 *         // Release the resource back to the pool
 *         fiftyoneDegreesPoolItemRelease(item);
 *     }
 *
 *     // Free the pool of resources
 *     fiftyoneDegreesPoolFree(&pool);
 * }
 * ```
 *
 * ## Design
 *
 * To improve performance in multi-threaded operation a non locking stack is 
 * used where a Compare and Swap (CAS) atomic operation is used to pop and push 
 * handles on and off the stack. The design was adapted from the following 
 * article (http://nullprogram.com/blog/2014/09/02/) which explains some of the 
 * challenges involved including the ABA problem 
 * (https://en.wikipedia.org/wiki/ABA_problem). It is for this reason the head 
 * structure is implemented as a union between the values and the exchange
 * integer. Pointers are not used as the address space for the stack is 
 * continuous and always very small compared to the total addressable memory 
 * space.
 *
 * @{
 */

#include <stdio.h>
#include <errno.h>
#include <stdint.h>
#ifdef _MSC_VER
#pragma warning (push)
#pragma warning (disable: 5105) 
#include <windows.h>
#pragma warning (default: 5105) 
#pragma warning (pop)
#endif
#include <assert.h>
#include <limits.h>
#include "data.h"
#include "exceptions.h"
#include "memory.h"
#ifndef FIFTYONE_DEGREES_NO_THREADING
#include "threading.h"
#endif
#include "common.h"

 /** @cond FORWARD_DECLARATIONS */
typedef struct fiftyone_degrees_pool_item_t fiftyoneDegreesPoolItem;
typedef struct fiftyone_degrees_pool_t fiftyoneDegreesPool;
/** @endcond */

/**
 * Used to create a new resource for use in the pool.
 * @param pool to create the resource for
 * @param state pointer to data used by the method
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return pointer to the new resource
 */
typedef void*(*fiftyoneDegreesPoolResourceCreate)(
	fiftyoneDegreesPool *pool,
	void *state,
	fiftyoneDegreesException *exception);

/**
 * Used to determine the additional size beyond the pointer used for each 
 * resource added to the pool.
 * @param state pointer to a state containing the resource and any additional
 * data needed for the calculation
 * @return addition size to allocate per resource
 */
typedef size_t(*fiftyoneDegreesPoolResourceSize)(void *state);

/**
 * Frees a resource previously created with #fiftyoneDegreesPoolResourceCreate.
 * @param pool containing the resource
 * @param resource to free
 */
typedef void(*fiftyoneDegreesPoolResourceFree)(
	fiftyoneDegreesPool *pool, 
	void *resource);

/**
 * Pool item node in the stack of items.
 */
typedef struct fiftyone_degrees_pool_item_t {
	void *resource; /**< Pointer to the resource in the pool */
	uint16_t next; /**< The next item in the stack */
	fiftyoneDegreesPool *pool; /**< Reader the handle belongs to */
} fiftyoneDegreesPoolItem;

/**
 * The head of the stack used for pop and push operations.
 */
typedef union fiftyone_degrees_pool_head_t {
	volatile long exchange; /**< Number used in the compare exchange operation */
	struct {
		uint16_t index; /**< Index of the item in the linked list */
		uint16_t aba; /**< ABA value used to ensure proper operation */
	} values; /**< Value index with its ABA value */
} fiftyoneDegreesPoolHead;

/**
 * Pool of resources stored as items in a stack.
 */
typedef struct fiftyone_degrees_pool_t {
	fiftyoneDegreesPoolItem *stack; /**< Pointer to the memory used by the
									    stack */
	fiftyoneDegreesPoolHead head; /**< Head of the stack */
	uint16_t count; /**< Number of resource items that stack can hold */
	fiftyoneDegreesPoolResourceFree resourceFree; /**< Frees a resource */
} fiftyoneDegreesPool;

/**
 * Initialises a pool data structure to support the number of concurrent
 * requests that can be made to the pool for resources that can be reused. 
 * The resourceCreate method is used to create a new resource for use in the 
 * pool which will be freed during when the release method is called on the
 * pool using the resourceFree method.
 * @param pool data structure to be initialised.
 * @param concurrency the number of resources the pool should contain.
 * @param state passed to the create resource method.
 * @param resourceCreate method used to create the resource to be added to 
 * items in the pool.
 * @param resourceFree method used to free a resource from the pool when the 
 * pool is freed.
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return a pointer to the pool if successful, otherwise NULL.
 */
EXTERNAL fiftyoneDegreesPool* fiftyoneDegreesPoolInit(
	fiftyoneDegreesPool *pool,
	uint16_t concurrency,
	void *state,
	fiftyoneDegreesPoolResourceCreate resourceCreate,
	fiftyoneDegreesPoolResourceFree resourceFree,
	fiftyoneDegreesException *exception);

/**
 * Gets the next free item from the pool for exclusive use by the caller. Every 
 * item returned must be released when the caller has finished with it using 
 * the #fiftyoneDegreesPoolItemRelease method.
 * @param pool to return items from.
 * @param exception pointer to an exception data structure to be used if an 
 * exception occurs. See exceptions.h.
 * @return the next free item, or NULL if no items are available.
 */
EXTERNAL fiftyoneDegreesPoolItem* fiftyoneDegreesPoolItemGet(
	fiftyoneDegreesPool *pool,
	fiftyoneDegreesException *exception);

/**
 * Releases the item back to the pool it belongs ready to be reused by another
 * operation.
 * @param item to be released back to the pool
 */
EXTERNAL void fiftyoneDegreesPoolItemRelease(fiftyoneDegreesPoolItem* item);

/**
 * Releases the items used by the pool freeing the resources used by each
 * item by calling the resourceFree method provided when the pool was created.
 * @param pool to be freed
 */
EXTERNAL void fiftyoneDegreesPoolFree(fiftyoneDegreesPool* pool);

/**
 * Resets the pool without releasing any resources.
 * @param pool to be reset
 */
EXTERNAL void fiftyoneDegreesPoolReset(fiftyoneDegreesPool *pool);

/**
 * @}
 */

#endif