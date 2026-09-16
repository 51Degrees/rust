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

#include "cache.h"
#include "fiftyone.h"

/**
 * Uncomment the following macro to enable cache validation. Very slow and 
 * only designed to be used when making changes to the cache logic.
 */
// #define FIFTYONE_DEGREES_CACHE_VALIDATE

/**
 * Validates the shard by checking the number of entries in the linked list and
 * the tree. Used by assert statements to validate the integrity of the cache
 * during development. Should not be compiled in release builds.
 * @param cache pointer to the cache being validated.
 * @return always return 0 as the purpose is to execute asserts in debug builds.
 */
#ifdef FIFTYONE_DEGREES_CACHE_VALIDATE

static void cacheValidateShard(CacheShard *shard) {
	uint32_t linkedListEntriesForward = 0;
	uint32_t linkedListEntriesBackwards = 0;
	uint32_t binaryTreeEntries = 0;
	CacheNode *node;

	// Check the list from first to last.
	node = shard->first;
	while (node != NULL &&
		linkedListEntriesForward <= shard->allocated) {
		linkedListEntriesForward++;
		node = node->listNext;
		assert(linkedListEntriesForward <= shard->allocated &&
			linkedListEntriesForward >= 0);
	}

	// Check the list from last to first.
	node = shard->last;
	while (node != NULL &&
		linkedListEntriesBackwards <= shard->allocated) {
		linkedListEntriesBackwards++;
		node = node->listPrevious;
		assert(linkedListEntriesBackwards <= shard->allocated ||
			linkedListEntriesBackwards >= 0);
	}

	// Check the binary tree. We need to remove one because the root
	// node doesn't contain any data.
	binaryTreeEntries = fiftyoneDegreesTreeCount(&shard->root);
	assert(binaryTreeEntries == shard->allocated ||
		binaryTreeEntries == shard->allocated - 1);
}

static int cacheValidate(const Cache *cache) {
	uint16_t i = 0;
	for (i = 0; i < cache->concurrency; i++) {
		cacheValidateShard(&cache->shards[i]);
	}
	return true;
}

#endif

/**
 * Initialise a newly allocated cache shard.
 * @param shard to initialise
 */
static void cacheInitShard(CacheShard *shard) {
	uint32_t i;
	CacheNode *current;

	// Initial shard is empty so set all pointers to null.
	fiftyoneDegreesTreeRootInit(&shard->root);
	shard->first = NULL;
	shard->last = NULL;

	// If single threading not used create a lock for exclusive access to the
	// shard.
#ifndef FIFTYONE_DEGREES_NO_THREADING
	FIFTYONE_DEGREES_MUTEX_CREATE(shard->lock);
#endif

	// Set the default values for an empty cache.
	for (i = 0; i < shard->capacity; i++) {
		current = &shard->nodes[i];
		current->shard = shard;
		fiftyoneDegreesTreeNodeInit(&current->tree, &shard->root);
		fiftyoneDegreesDataReset(&current->data);
		current->listNext = NULL;
		current->listPrevious = NULL;
		current->activeCount = 0;
	}
}

/**
 * Gets the capacity of each shard in the cache.
 * @param capacity minimum capacity for the cache overall
 * @param concurrency the expected maximum number of concurrent accesses
 */
static uint32_t cacheShardCapacity(uint32_t capacity, uint16_t concurrency) {
	return (capacity % concurrency) + (capacity /concurrency);
}

/**
 * Initialises the cache by setting pointers for the linked list and binary
 * tree.
 * @param cache pointer to the cache to be initialised
 */
static void cacheInit(Cache *cache) {
	uint16_t i;
	CacheShard *shard;
	for (i = 0; i < cache->concurrency; i++) {
		shard = &cache->shards[i];
		shard->cache = cache;
		shard->capacity = cache->capacity / cache->concurrency;
		shard->allocated = 0;
		shard->nodes = &cache->nodes[shard->capacity * i];
		cacheInitShard(shard);
	}
}

/**
 * CACHE METHODS
 */

/**
 * Removes the node from the linked list if it was present in a linked list. 
 * @param node the node to be removed from it's shard's linked list
 */
static void cacheRemoveFromList(CacheNode *node) {
	if (node->listNext != NULL) {
		node->listNext->listPrevious = node->listPrevious;
	}
	else {
		node->shard->last = node->listPrevious;
	}
	if (node->listPrevious != NULL) {
		node->listPrevious->listNext = node->listNext;
	}
	else {
		node->shard->first = node->listNext;
	}
	node->listNext = NULL;
	node->listPrevious = NULL;
}

/**
 * Increases the active count for the node and if it was not in use already
 * (i.e. the first time being returned, or sitting idly in the shard's linked
 * list) remove it from the linked list associated with the shard so it is not
 * available for write operations.
 * @param node the node to be incremented and removed from it's shard's linked
 * list
 */
static void cacheIncremenetCheckAndRemove(CacheNode *node) {
	node->activeCount++;
	if (node->activeCount == 1) {
		cacheRemoveFromList(node);
	}
}

/**
 * Adds the node into the linked list. This is added at the head of the list
 * as it is now the most recently used.
 * @param node to add
 */
static void cacheAddToHead(CacheNode *node) {
	CacheShard *shard = node->shard;
	assert(node->listNext == NULL);
	assert(node->listPrevious == NULL);
	node->listNext = shard->first;
	if (shard->first != NULL) {
		shard->first->listPrevious = node;
	}

	shard->first = node;

	if (shard->last == NULL) {
		shard->last = shard->first;
	}

	// Validate the state of the list.
	assert(node->shard->first == node);
	assert(node->shard->first->listPrevious == NULL);
	assert(node->shard->last->listNext == NULL);
}

/**
 * Returns the next free node from the cache which can be used to add a
 * new entry to. Once the cache is full then the node returned is the one
 * at the end of the linked list which will contain the least recently used
 * data.
 * @param shard to return the next free node from.
 * @return a pointer to a free node.
 */
static CacheNode *cacheGetNextFree(CacheShard *shard) {
	#ifdef FIFTYONE_DEGREES_CACHE_VALIDATE
	int countBefore, countAfter;
	#endif

	CacheNode *node; // The oldest node in the shard.

	if (shard->allocated < shard->capacity) {
		// Return the free element at the end of the cache and update
		// the number of allocated elements.
		node = &shard->nodes[shard->allocated++];
	}
	else {
		// Use the oldest element in the list.
		node = shard->last;

		if (node == NULL) {
			// There are no available nodes to return, so return null.
			return NULL;
		}

		// Remove the last element from the list as it's about to be populated.
		assert(node->activeCount == 0);
		cacheRemoveFromList(node);

		// Remove the last result from the binary tree.
		#ifdef FIFTYONE_DEGREES_CACHE_VALIDATE
		countBefore = TreeCount(&shard->root);
		#endif
		TreeDelete(&node->tree);
		#ifdef FIFTYONE_DEGREES_CACHE_VALIDATE
		countAfter = TreeCount(&shard->root);
		assert(countBefore - 1 == countAfter);
		#endif
	}

	// Set the pointers of the node to null indicating that the
	// entry is not part of the dictionary anymore.
	TreeNodeRemove(&node->tree);

	return node;
}

/**
 * Loads the data for the key into the least frequently used node in the shard
 * if one is available.
 * @param shard dictated by the key
 * @param key to get or load
 * @return pointer to the node with data for the key, or NULL if there are no 
 * free nodes
 */
static CacheNode* cacheLoad(
	CacheShard *shard,
	const void *key,
	int64_t keyHash,
	Exception *exception) {
	CacheNode *node = cacheGetNextFree(shard);
	if (node != NULL) {
		node->activeCount = 1;

		// Load the data into then node setting the valid flag to indicate if
		// the item was loaded correctly.
		shard->cache->load(
			shard->cache->loaderState,
			&node->data,
			key,
			exception);

		// If not exception then add the node to the head of the tree. The key
		// hash was already computed by the caller, so reuse it rather than
		// invoking the (potentially expensive) hash function a second time.
		if (EXCEPTION_OKAY) {
			node->tree.key = keyHash;
			TreeInsert(&node->tree);
		}
	}
	return node;
}

/**
 * Free the data containing in the cache shard.
 * @param shard to free
 */
static void cacheShardFree(CacheShard *shard) {
	uint32_t i;
	CacheNode *node;
	for (i = 0; i < shard->capacity; i++) {
		node = &shard->nodes[i];
		if (node->data.ptr != NULL && node->data.allocated > 0) {
			Free(node->data.ptr);
			DataReset(&node->data);
		}
	}
#ifndef FIFTYONE_DEGREES_NO_THREADING
	FIFTYONE_DEGREES_MUTEX_CLOSE(shard->lock);
#endif
}

/**
 * EXTERNAL CACHE METHODS
 */

fiftyoneDegreesCache* fiftyoneDegreesCacheCreate(
	uint32_t capacity,
	uint16_t concurrency,
	fiftyoneDegreesCacheLoadMethod load,
	fiftyoneDegreesCacheHashCodeMethod hash,
	const void *state) {
	size_t cacheSize, nodesSize, shardsSize;
	Cache *cache;

	// The capacity of each shard in the cache must allow for a minimum of 
	// one entry for each thread that could access the shard.
	if (concurrency == 0 || capacity / concurrency < concurrency) {
		return NULL;
	}

	// Work out the total amount of memory used by the cache. Keep the list
	// of nodes and header together so they're in the same continuous memory
	// space and can be allocated and freed in a single operation.
	shardsSize = sizeof(CacheShard) * concurrency;
	nodesSize = sizeof(CacheNode) * 
		cacheShardCapacity(capacity, concurrency) * concurrency;
	cacheSize = sizeof(Cache) + shardsSize + nodesSize;
	cache = (Cache*)Malloc(cacheSize);
	if (cache != NULL) {

		// The shards are set to the byte after the header.
		cache->shards = (CacheShard*)(cache + 1);

		// The nodes are set to the byte after the shards.
		cache->nodes = (CacheNode*)(cache->shards + concurrency);

		// Set the parameters for the cache.
		cache->load = load;
		cache->hash = hash;
		cache->loaderState = state;
		cache->hits = 0;
		cache->misses = 0;
		cache->concurrency = concurrency;
		cache->capacity =
			cacheShardCapacity(capacity, concurrency) * concurrency;

		// Initialise the linked lists and binary tree.
		cacheInit(cache);
	}
	// Check the cache if in debug mode.
	assert(cache != NULL);
#ifdef FIFTYONE_DEGREES_CACHE_VALIDATE
	assert(cacheValidate(cache));
#endif

	return cache;
}

void fiftyoneDegreesCacheFree(fiftyoneDegreesCache *cache) {
	uint16_t i;

	// Free any data items that are created and are marked to be freed by the
	// cache and shards.
	for (i = 0; i < cache->concurrency; i++) {
		cacheShardFree(&cache->shards[i]);
	}

	// Finally free all the memory used by the cache.
	Free(cache);
}

fiftyoneDegreesCacheNode* fiftyoneDegreesCacheGet(
	fiftyoneDegreesCache *cache, 
	const void *key,
	fiftyoneDegreesException *exception) {
	CacheNode *node;
	int64_t keyHash = cache->hash(key);
	CacheShard *shard = &cache->shards[abs((int)keyHash) % cache->concurrency];

#ifndef FIFTYONE_DEGREES_NO_THREADING
	FIFTYONE_DEGREES_MUTEX_LOCK(&shard->lock);
#endif

	// Check if the key already exists in the cache shard.
	node = (CacheNode*)fiftyoneDegreesTreeFind(&shard->root, keyHash);
	if (node != NULL) {

		// The node was found in the cache, so increment the active count and
		// remove from the shard's linked list if required.
		cacheIncremenetCheckAndRemove(node);
		cache->hits++;
	}
	else {

		// The key does not exist so load it, reusing the hash already computed.
		node = cacheLoad(shard, key, keyHash, exception);
		cache->misses++;
	}

#ifndef FIFTYONE_DEGREES_NO_THREADING
	FIFTYONE_DEGREES_MUTEX_UNLOCK(&shard->lock);
#endif

	assert(node == NULL || node->activeCount > 0);

	return node;
}

void fiftyoneDegreesCacheRelease(fiftyoneDegreesCacheNode* node) {
	// Decrement the active count for the node and check if it's now zero. If
	// it isn't then move it to the head of the linked list as the most
	// recently used node.
#ifndef FIFTYONE_DEGREES_NO_THREADING
	FIFTYONE_DEGREES_MUTEX_LOCK(&node->shard->lock);
#endif
	assert(node->activeCount != 0);
	node->activeCount--;
	if (node->activeCount == 0) {
		cacheAddToHead(node);
	}
#ifndef FIFTYONE_DEGREES_NO_THREADING
	FIFTYONE_DEGREES_MUTEX_UNLOCK(&node->shard->lock);
#endif
}

int64_t fiftyoneDegreesCacheHash32(const void *key) {
	return (int64_t)(*(int32_t*)key);
}

int64_t fiftyoneDegreesCacheHash64(const void *key) {
	return *(int64_t*)key;
}