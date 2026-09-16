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

#include "memory.h"
#include "fiftyone.h"

#ifndef FIFTYONE_DEGREES_MEMORY_TRACKER_SHARDS
#ifndef FIFTYONE_DEGREES_NO_THREADING
#define FIFTYONE_DEGREES_MEMORY_TRACKER_SHARDS 20
#else
#define FIFTYONE_DEGREES_MEMORY_TRACKER_SHARDS 1
#endif
#else 
#endif

static int shardDivider = sizeof(void*) * 3;

static bool initialised = false;

typedef struct memory_allocation_t {
	fiftyoneDegreesTreeNode tree; /* Tree node data structure */
	size_t size; /* The amount of memory allocated at pointer */
} allocation;

typedef struct memory_allocation_tree_t {
	fiftyoneDegreesTreeRoot roots[FIFTYONE_DEGREES_MEMORY_TRACKER_SHARDS];
#ifndef FIFTYONE_DEGREES_NO_THREADING
	fiftyoneDegreesMutex locks[FIFTYONE_DEGREES_MEMORY_TRACKER_SHARDS];
	fiftyoneDegreesMutex lock;
#endif
	size_t allocated;
	size_t max;
} allocationTree;

static allocationTree state;

bool fiftyoneDegreesMemoryAdvance(
	fiftyoneDegreesMemoryReader *reader,
	size_t advanceBy) {
	if (reader == NULL || reader->current == NULL) {
		return false;
	}
	reader->current += advanceBy;
	if (reader->current > reader->lastByte) {
		return false;
	}
	return true;
}

void* fiftyoneDegreesMemoryStandardMalloc(size_t size) {
	return malloc(size);
}

void* fiftyoneDegreesMemoryStandardMallocAligned(int alignment, size_t size) {
	size += size % alignment;
#if defined( _MSC_VER) || defined(__MINGW32__) || defined(__MINGW64__)
	return _aligned_malloc(size, alignment);
#elif defined (__APPLE__)
	void *pointer;
	if (posix_memalign(&pointer, alignment, size) == 0) {
		return pointer;
	}
	else {
		return NULL;
	}

#else
	return aligned_alloc(alignment, size);
#endif
}

static int getShardFromPointer(void *pointer) {
#ifdef __GNUC__
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wpointer-to-int-cast"
#endif
	return (((uint64_t)pointer) / shardDivider) %
		FIFTYONE_DEGREES_MEMORY_TRACKER_SHARDS;
#ifdef __GNUC__
#pragma GCC diagnostic pop
#endif
}

static void tryInit() {
	if (initialised == false) {
		MemoryTrackingReset();
	}
}

static void trackAllocation(void* pointer, size_t size) {
	// Create a new tree node to record the allocation.
	allocation* record = (allocation*)malloc(sizeof(allocation));
	int shard = getShardFromPointer(pointer);
	assert(record != NULL);
	assert(shard < FIFTYONE_DEGREES_MEMORY_TRACKER_SHARDS);
	fiftyoneDegreesTreeNodeInit(&record->tree, &state.roots[shard]);
#ifdef __GNUC__
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wpointer-to-int-cast"
#endif
	record->tree.key = (int64_t)pointer;
#ifdef __GNUC__
#pragma GCC diagnostic pop
#endif
	record->size = size;

	// Update the tracking tree with the new allocation.
#ifndef FIFTYONE_DEGREES_NO_THREADING
	FIFTYONE_DEGREES_MUTEX_LOCK(&state.locks[shard]);
#endif
	fiftyoneDegreesTreeInsert(&record->tree);
#ifndef FIFTYONE_DEGREES_NO_THREADING
	FIFTYONE_DEGREES_MUTEX_UNLOCK(&state.locks[shard]);
#endif

	// Update the allocated and max values.
#ifndef FIFTYONE_DEGREES_NO_THREADING
	FIFTYONE_DEGREES_MUTEX_LOCK(&state.lock);
#endif
	state.allocated += size;
	if (state.allocated > state.max) {
		state.max = state.allocated;
	}
#ifndef FIFTYONE_DEGREES_NO_THREADING
	FIFTYONE_DEGREES_MUTEX_UNLOCK(&state.lock);
#endif

}

static void untrackAllocation(void *pointer) {
	size_t size;
	int shard = getShardFromPointer(pointer);

	// Get the size of the memory being freed and free the tracking memory.
#ifndef FIFTYONE_DEGREES_NO_THREADING
	FIFTYONE_DEGREES_MUTEX_LOCK(&state.locks[shard]);
#endif
#ifdef __GNUC__
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wpointer-to-int-cast"
#endif
	allocation* record = (allocation*)fiftyoneDegreesTreeFind(
		&state.roots[shard],
		(int64_t)pointer);
#ifdef __GNUC__
#pragma GCC diagnostic pop
#endif
	assert(record != NULL);
	size = record->size;
	fiftyoneDegreesTreeDelete(&record->tree);
#ifndef FIFTYONE_DEGREES_NO_THREADING
	FIFTYONE_DEGREES_MUTEX_UNLOCK(&state.locks[shard]);
#endif
	free(record);

#ifndef FIFTYONE_DEGREES_NO_THREADING
	FIFTYONE_DEGREES_MUTEX_LOCK(&state.lock);
#endif
	state.allocated -= size;
#ifndef FIFTYONE_DEGREES_NO_THREADING
	FIFTYONE_DEGREES_MUTEX_UNLOCK(&state.lock);
#endif

}
void* fiftyoneDegreesMemoryTrackingMallocAligned(
	int alignment,
	size_t size) {
	// Check that the tracker is initialised.
	tryInit();

	// Allocate the memory.
	void* pointer = MemoryStandardMallocAligned(
		alignment,
		size);

	// Track the allocation.
	trackAllocation(pointer, size + size % alignment);
	return pointer;
}

void* fiftyoneDegreesMemoryTrackingMalloc(size_t size) {

	// Check that the tracker is initialised.
	tryInit();

	// Allocate the memory.
	void* pointer = MemoryStandardMalloc(size);

	// Track the allocation.
	trackAllocation(pointer, size);
	return pointer;
}

void fiftyoneDegreesMemoryStandardFree(void *pointer) {
	free(pointer);
}

void fiftyoneDegreesMemoryStandardFreeAligned(void* pointer) {
#if defined( _MSC_VER) || defined(__MINGW32__) || defined(__MINGW64__)
	_aligned_free(pointer);
#else
	free(pointer);
#endif
}

void fiftyoneDegreesMemoryTrackingFree(void *pointer) {
	untrackAllocation(pointer);
	// Finally free the pointer.
	MemoryStandardFree(pointer);
}

void fiftyoneDegreesMemoryTrackingFreeAligned(void* pointer) {
	untrackAllocation(pointer);
	// Finally free the pointer.
	MemoryStandardFreeAligned(pointer);
}

size_t fiftyoneDegreesMemoryTrackingGetMax() {
	return state.max;
}

size_t fiftyoneDegreesMemoryTrackingGetAllocated() {
	return state.allocated;
}

void fiftyoneDegreesMemoryTrackingReset() {
	int i;
	if (initialised == false) {
		for (i = 0; i < FIFTYONE_DEGREES_MEMORY_TRACKER_SHARDS; i++) {
#ifndef FIFTYONE_DEGREES_NO_THREADING
			FIFTYONE_DEGREES_MUTEX_CREATE(state.locks[i]);
#endif
			fiftyoneDegreesTreeRootInit(&state.roots[i]);
		}
#ifndef FIFTYONE_DEGREES_NO_THREADING
		FIFTYONE_DEGREES_MUTEX_CREATE(state.lock);
#endif
	}
	state.allocated = 0;
	state.max = 0;
	initialised = true;
}

void fiftyoneDegreesSetUpMemoryTracking() {
	fiftyoneDegreesMemoryTrackingReset();
	fiftyoneDegreesMalloc = fiftyoneDegreesMemoryTrackingMalloc;
	fiftyoneDegreesMallocAligned = fiftyoneDegreesMemoryTrackingMallocAligned;
	fiftyoneDegreesFree = fiftyoneDegreesMemoryTrackingFree;
	fiftyoneDegreesFreeAligned = fiftyoneDegreesMemoryTrackingFreeAligned;
}

size_t fiftyoneDegreesUnsetMemoryTracking() {
	size_t memAlloced = MemoryTrackingGetAllocated();
	fiftyoneDegreesMalloc = fiftyoneDegreesMemoryStandardMalloc;
	fiftyoneDegreesMallocAligned = fiftyoneDegreesMemoryStandardMallocAligned;
	fiftyoneDegreesFree = fiftyoneDegreesMemoryStandardFree;
	fiftyoneDegreesFreeAligned = fiftyoneDegreesMemoryStandardFreeAligned;
	fiftyoneDegreesMemoryTrackingReset();
	return memAlloced;
}

#ifdef FIFTYONE_DEGREES_MEMORY_TRACK_ENABLED

/**
 * Enable memory tracking.
 */

void *(FIFTYONE_DEGREES_CALL_CONV *fiftyoneDegreesMalloc)(size_t size) =
fiftyoneDegreesMemoryTrackingMalloc;

void* (FIFTYONE_DEGREES_CALL_CONV* fiftyoneDegreessMallocAligned)(
	int alignment,
	size_t size) = fiftyoneDegreesMemoryTrackingMallocAligned;

void (FIFTYONE_DEGREES_CALL_CONV *fiftyoneDegreesFree)(void* pointer) =
fiftyoneDegreesMemoryTrackingFree;

void (FIFTYONE_DEGREES_CALL_CONV *fiftyoneDegreesFreeAligned)(void* pointer) =
fiftyoneDegreesMemoryTrackingFreeAligned;

#else

/**
 * Disable memory tracking.
 */

void* (FIFTYONE_DEGREES_CALL_CONV* fiftyoneDegreesMalloc)(size_t size) =
fiftyoneDegreesMemoryStandardMalloc;

void* (FIFTYONE_DEGREES_CALL_CONV* fiftyoneDegreesMallocAligned)(
	int alignment,
	size_t size) = fiftyoneDegreesMemoryStandardMallocAligned;

void (FIFTYONE_DEGREES_CALL_CONV *fiftyoneDegreesFree)(void *pointer) =
fiftyoneDegreesMemoryStandardFree;

void (FIFTYONE_DEGREES_CALL_CONV *fiftyoneDegreesFreeAligned)(void* pointer) =
fiftyoneDegreesMemoryStandardFreeAligned;

#endif