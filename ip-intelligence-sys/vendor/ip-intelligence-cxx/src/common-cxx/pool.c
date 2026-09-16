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

#include "pool.h"
#include "fiftyone.h"

fiftyoneDegreesPool* fiftyoneDegreesPoolInit(
	fiftyoneDegreesPool *pool,
	uint16_t concurrency,
	void *state,
	fiftyoneDegreesPoolResourceCreate resourceCreate,
	fiftyoneDegreesPoolResourceFree resourceFree,
	fiftyoneDegreesException *exception) {
	uint16_t i = 1;
	PoolItem *item;

	// Add one to the concurrency value so that a NULL marker can be 
	// written as the last item in the linked list that if returned
	// indicates that the concurrency has been exceeded.
	uint16_t listItems = concurrency + 1;

	// Set the stack and head of the linked list.
	pool->count = 0;
	pool->head.exchange = 0;
	pool->resourceFree = resourceFree;

	// Allocate memory for the stack.
	pool->stack = (PoolItem*)Malloc(sizeof(PoolItem) * listItems);
	if (pool->stack != NULL) {

		// The entry at index 0 in the stack is the null item which if ever
		// retrieved indicates that the pool is exhausted. It does not contain
		// a value resource.
		item = &pool->stack[0];
		item->pool = pool;
		item->resource = NULL;

		// Initialise all the resources in the pool after the null terminator.
		while (i < listItems && EXCEPTION_OKAY) {
			item = &pool->stack[i];
			item->pool = pool;
			item->resource = resourceCreate(pool, state, exception);
			pool->count++;
			item->next = pool->head.values.index;
			pool->head.values.index = i;
			i++;
		}
	}
	else {
		EXCEPTION_SET(INSUFFICIENT_MEMORY);
	}

	return pool;
}

fiftyoneDegreesPoolItem* fiftyoneDegreesPoolItemGet(
	fiftyoneDegreesPool *pool,
	fiftyoneDegreesException *exception) {
	PoolHead orig;
#ifndef FIFTYONE_DEGREES_NO_THREADING
	PoolHead next;
	do {
#endif
		orig = pool->head;

		// Check that the head of the list is not the null resource which
		// would indicate that there are more active concurrent operations than 
		// the pool has been configured for.
		if (pool->stack[orig.values.index].resource == NULL) {
			EXCEPTION_SET(INSUFFICIENT_HANDLES)
			return NULL;
		}

#ifndef FIFTYONE_DEGREES_NO_THREADING
		next.values.aba = orig.values.aba + 1;
		next.values.index = pool->stack[orig.values.index].next;
	} while (INTERLOCK_EXCHANGE(
		pool->head.exchange,
		next.exchange,
		orig.exchange) != orig.exchange);
#else 
		pool->head.values.index = pool->stack[orig.values.index].next;
#endif
	return &pool->stack[orig.values.index];
}

void fiftyoneDegreesPoolItemRelease(fiftyoneDegreesPoolItem *item) {
#ifndef FIFTYONE_DEGREES_NO_THREADING
	PoolHead orig, next;
	do {
		orig = item->pool->head;
		item->next = orig.values.index;
		next.values.aba = orig.values.aba + 1;
		next.values.index = (uint16_t)(item - item->pool->stack);
	} while (INTERLOCK_EXCHANGE(
		item->pool->head.exchange,
		next.exchange,
		orig.exchange) != orig.exchange);
#else
	item->next = item->pool->head.values.index;
	item->pool->head.values.index =
		(uint16_t)(item - item->pool->stack);
#endif
}

void fiftyoneDegreesPoolReset(fiftyoneDegreesPool *pool) {
	pool->head.values.index = 0;
	pool->head.values.aba = 0;
	pool->stack = NULL;
	pool->count = 0;
	pool->resourceFree = NULL;
}

void fiftyoneDegreesPoolFree(fiftyoneDegreesPool *pool) {
	void *resource;
	uint16_t i;
	if (pool->stack != NULL) {
		if (pool->resourceFree != NULL) {
			for (i = 0; i <= pool->count; i++) {
				resource = pool->stack[i].resource;
				if (resource != NULL) {
					pool->resourceFree(pool, resource);
				}
			}
		}
		Free(pool->stack);
	}
	PoolReset(pool);
}