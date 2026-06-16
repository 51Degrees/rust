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
 * @defgroup FiftyOneDegreesCache Cache
 *
 * Fixed size, thread safe, loading, tree based cache.
 *
 * ## Introduction
 *
 * Implements a fixed size, thread safe, loading, tree based cache. Items
 * requested which are not in the cache already are loaded using the specified
 * load method, before returning as with items which are already in the cache.
 *
 * Items are fetched from the cache using the #fiftyoneDegreesCacheGet method
 * and the item is prevented from being expelled from the cache until the 
 * #fiftyoneDegreesCacheRelease method is called. Failure to release cache
 * items once they are finished with will result in the available nodes in the
 * cache being exhausted (i.e. no new items can be loaded into the cache).
 *
 * As the cache is fixed size, the size must be set correctly in order to
 * prevent failures in the get method. The size of a cache **MUST** be equal to
 * or greater than the maximum number of items which will be in use
 * simultaneously across all threads. Fetching more items than the cache was
 * created to expect will result in the same failure as not releasing items.
 *
 * The cache is sharded based on the modulo of the key to improve performance
 * in multi threaded operation where an even distribution of key modulos are
 * present.
 *
 * Details of the red black tree implementation can be found in tree.c.
 *
 * ## Example Usage
 *
 * ```
 * byte *data;
 * fiftyoneDegreesCacheLoadMethod *methodToLoadEntryFromData;
 *
 * // Create a cache
 * fiftyoneDegreesCache cache = fiftyoneDegreesCacheCreate(
 *      100,
 *      1,
 *      methodToLoadEntryFromData,
 *      fiftyoneDegreesCacheHash64,
 *      data);
 *
 * // Get an item from the cache
 * fiftyoneDegreesCacheNode *entry = fiftyoneDegreesCacheGet(
 *     cache,
 *     &0,
 *     exception);
 *
 * // Check that the value was found
 * if (entry != NULL) {
 *
 *     // Get the value from the entry
 *     int *value = (int*)entry->data.ptr;
 *
 *     // Do something with the value
 *     // ...
 *
 *     // Release the entry back to the cache
 *     fiftyoneDegreesCacheRelease(entry);
 * }
 *
 * // Free the cache
 * fiftyoneDegreesCacheFree(cache);
 * ```
 * @{
 */

#ifndef FIFTYONE_DEGREES_CACHE_H_INCLUDED
#define FIFTYONE_DEGREES_CACHE_H_INCLUDED

/* Define NDEBUG if needed, to ensure asserts are disabled in release builds */
#if !defined(DEBUG) && !defined(_DEBUG) && !defined(NDEBUG)
#define NDEBUG
#endif

#include <stdint.h>
#include <stdbool.h>

#ifdef _MSC_VER
#endif
#ifdef _MSC_VER
#pragma warning (push)
#pragma warning (disable: 5105) 
#include <windows.h>
#pragma warning (default: 5105) 
#pragma warning (pop)
#endif
#include <assert.h>
#include "data.h"
#include "exceptions.h"
#include "tree.h"
#include "common.h"

#ifndef FIFTYONE_DEGREES_NO_THREADING
#include "threading.h"
#endif

/** @cond FORWARD_DECLARATIONS */
typedef struct fiftyone_degrees_cache_node_t fiftyoneDegreesCacheNode;
typedef struct fiftyone_degrees_cache_shard_t fiftyoneDegreesCacheShard;
typedef struct fiftyone_degrees_cache_t fiftyoneDegreesCache;
/** @endcond */

/**
 * Cache node structure used for storing data in the cache along with
 * its key.
 */
typedef struct fiftyone_degrees_cache_node_t {
	fiftyoneDegreesTreeNode tree; /**< Tree node for this cache node */
	fiftyoneDegreesData data; /**< Data contained in the node */
	fiftyoneDegreesCacheShard *shard; /**< Shard the node is associated with */
	fiftyoneDegreesCacheNode *listPrevious; /**< Previous node or NULL if first */
	fiftyoneDegreesCacheNode *listNext; /**< Next node or NULL if last */
	int activeCount; /**< Number of external references to the node data */
} fiftyoneDegreesCacheNode;

/**
 * Cache shard structure used to enable concurrent access to the cache.
 */
typedef struct fiftyone_degrees_cache_shard_t {
	fiftyoneDegreesCache *cache; /**< Pointer to the cache to which the node
								     belongs */
	fiftyoneDegreesTreeRoot root; /**< Root node of the red black tree */
	uint32_t capacity; /**< Capacity of the shard */
	uint32_t allocated; /**< Number of nodes currently used in the shard */
	fiftyoneDegreesCacheNode *nodes; /**< Pointer to the array of all nodes */
	fiftyoneDegreesCacheNode *first; /**< Pointer to the first node in the
									     linked list */
	fiftyoneDegreesCacheNode *last; /**< Pointer to the last node in the
									    linked list */
#ifndef FIFTYONE_DEGREES_NO_THREADING
	fiftyoneDegreesMutex lock; /**< Used to ensure exclusive access to the
								   shard for get and release operations */
#endif
} fiftyoneDegreesCacheShard;

/**
 * Method used to load data into the cache.
 * @param state information used for the load operation.
 * @param data structure to be used to store the data loaded.
 * @param key for the item in the collection to be loaded.
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 */
typedef void(*fiftyoneDegreesCacheLoadMethod)(
	const void *state,
	fiftyoneDegreesData *data,
	const void *key,
	fiftyoneDegreesException *exception);

/**
 * Method used to calculate a hash code from the key.
 * @param key the data to be calculate the integer key.
 * @return 64 bit representation of the key data.
 */
typedef int64_t(*fiftyoneDegreesCacheHashCodeMethod)(const void* key);

/**
 * Cache structure to store the root of the red black tree and a list of
 * allocated cache nodes. This also contains cache metrics and pointers to
 * methods used when being used as a loading cache.
 */
typedef struct fiftyone_degrees_cache_t {
	fiftyoneDegreesCacheShard *shards; /**< Array of shards / concurrency */
	fiftyoneDegreesCacheNode *nodes; /**< Array of nodes / capacity */
	uint16_t concurrency; /**< Expected concurrency and number of shards */
	int32_t capacity; /**< Capacity of the cache */
	unsigned long hits; /**< The requests served from the cache */
	unsigned long misses; /**< The requests NOT served from the cache */
	fiftyoneDegreesCacheLoadMethod load; /**< Used by the cache to load an item
										 into the cache */
	fiftyoneDegreesCacheHashCodeMethod hash; /**< Used to hash a key pointer */
	const void* loaderState; /**< Cache loader specific state */
} fiftyoneDegreesCache;

/**
 * Creates a new cache.The cache must be destroyed with the
 * #fiftyoneDegreesCacheFree method.
 * @param capacity maximum number of items that the cache should store
 * @param concurrency the expected number of parallel operations
 * @param load pointer to method used to load an entry into the cache
 * @param hash pointer to a method used to hash the key into a int64_t
 * @param state pointer to state information to pass to the load method
 * @return a pointer to the cache created, or NULL if one was not created.
 */
EXTERNAL fiftyoneDegreesCache *fiftyoneDegreesCacheCreate(
	uint32_t capacity,
	uint16_t concurrency,
	fiftyoneDegreesCacheLoadMethod load,
	fiftyoneDegreesCacheHashCodeMethod hash,
	const void *state);

/**
 * Frees the cache structure, all allocated nodes and their data.
 * @param cache to be freed
 */
EXTERNAL void fiftyoneDegreesCacheFree(fiftyoneDegreesCache *cache);

/**
 * Gets an item from the cache. If an item is not in the cache, it is loaded
 * using the loader the cache was initialized with.
 *
 * The cache being used as a loading cache must have a load method defined
 * which returns a pointer to the data relating to the key used. This method
 * may, or may not, allocate memory or free memory previously allocated to
 * data in the cache node.
 *
 * Nodes fetched from the cache are protected from modification until all
 * references to them are released. This means that the size of the cache must
 * be carefully chosen to be no smaller than the maximum number of nodes which
 * may be in use at any one time. Attempting to fetch a node when there are no
 * free nodes to load the data into will result in a null being returned.
 *
 * @param cache to get the entry from
 * @param key for the item to be returned
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return pointer to the requested item or null if too many items have been 
 * fetched and not released or the key is not valid
 */
EXTERNAL fiftyoneDegreesCacheNode* fiftyoneDegreesCacheGet(
	fiftyoneDegreesCache *cache,
	const void *key,
	fiftyoneDegreesException *exception);

/**
 * Releases the cache node previous obtained via #fiftyoneDegreesCacheGet so 
 * that it can be evicted from the cache if needed.
 * @param node to be released.
 */
EXTERNAL void fiftyoneDegreesCacheRelease(fiftyoneDegreesCacheNode *node);

/**
 * Passed a pointer to a 32 bit / 4 byte data structure and returns the data as
 * a 64 bit / 8 byte value for use in the cache. Used when cache keys are 32 
 * bit integers.
 * @param key to be used in the cache
 * @return key represented as a 64 bit integer
 */
EXTERNAL int64_t fiftyoneDegreesCacheHash32(const void *key);

/**
 * Passed a pointer to a 64 bit / 8 byte data structure and returns the data as
 * a 64 bit / 8 byte value for use in the cache. Used when cache keys are 64
 * bit integers.
 * @param key to be used in the cache
 * @return key represented as a 64 bit integer
 */
EXTERNAL int64_t fiftyoneDegreesCacheHash64(const void *key);

/**
 * @}
 */
#endif
