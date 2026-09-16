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

#ifndef FIFTYONE_DEGREES_WEIGHTED_ITEM_H_INCLUDED
#define FIFTYONE_DEGREES_WEIGHTED_ITEM_H_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesWeightedItem Weighted Items
 *
 * A weighted collection item with an associated weight value.
 *
 * ## Introduction
 *
 * A WeightedItem represents a value retrieved from the dataset together with
 * a weight indicating the proportion or confidence of this value (out of 65535).
 * Used for profile-level weighting (from profile groups) where the weight
 * indicates the proportion of the matched IP range attributable to this profile,
 * or for value-level weighting where the weight is stored in the Value record.
 *
 * ## List Management
 *
 * WeightedItemList provides a resizable array of WeightedItem structures with
 * functions for initialization, release, extension, and adding items.
 *
 * @{
 */

#include <stdint.h>
#include "collection.h"
#include "common.h"
#include "status.h"
#include "exceptions.h"

/**
 * Default resize factor for WeightedItemList when extending capacity.
 */
#define FIFTYONE_DEGREES_WEIGHTED_ITEM_LIST_RESIZE_FACTOR 2

/**
 * Default load factor for WeightedItemList initial capacity calculation.
 */
#define FIFTYONE_DEGREES_WEIGHTED_ITEM_LIST_DEFAULT_LOAD_FACTOR 4

/**
 * A weighted collection item. Represents a value retrieved from the
 * dataset together with a weight indicating the proportion or
 * confidence of this value (out of 65535^2). Used for profile-level
 * weighting (from profile groups) where the weight indicates the
 * proportion of the matched IP range attributable to this profile,
 * or for value-level weighting stored in the Value record, or both.
 */
typedef struct fiftyone_degrees_weighted_item_t {
	fiftyoneDegreesCollectionItem item; /**< The collection item containing the value */
	uint32_t rawWeighting; /**< Weight as uint32_t (0-65535^2, representing 0.0-1.0).
						   Final/effective weight after multiplying weights
						   from both profile group (or "1") and value (or "1").*/
} fiftyoneDegreesWeightedItem;

/// Max value for WeightedItem.rawWeighting
#define FIFTYONE_DEGREES_WEIGHTED_ITEM_MAX_WEIGHT \
	((uint32_t)(((uint32_t)65535)*((uint32_t)65535)))

/**
 * A resizable list of weighted items.
 */
typedef struct fiftyone_degrees_weighted_item_list_t {
	fiftyoneDegreesWeightedItem *items; /**< Array of weighted items */
	uint32_t count; /**< Number of items currently in use */
	uint32_t capacity; /**< Allocated capacity of the items array */
	float loadFactor; /**< Load factor threshold for auto-extension */
} fiftyoneDegreesWeightedItemList;

/**
 * Initializes a weighted item list with the specified initial capacity.
 * @param list pointer to the list to initialize
 * @param initialCapacity the initial capacity to allocate
 * @param loadFactor the load factor threshold for auto-extension (0.0-1.0)
 */
EXTERNAL void fiftyoneDegreesWeightedItemListInit(
	fiftyoneDegreesWeightedItemList *list,
	uint32_t initialCapacity,
	float loadFactor);

/**
 * Releases all collection items in the list and resets the count to zero.
 * Does not free the items array itself. Each item's collection is obtained
 * from the item itself.
 * @param list pointer to the list to release
 */
EXTERNAL void fiftyoneDegreesWeightedItemListRelease(
	fiftyoneDegreesWeightedItemList *list);

/**
 * Frees the items array and resets the list to empty state.
 * Releases all items first.
 * @param list pointer to the list to free
 */
EXTERNAL void fiftyoneDegreesWeightedItemListFree(
	fiftyoneDegreesWeightedItemList *list);

/**
 * Extends the capacity of the list to the new capacity.
 * @param list pointer to the list to extend
 * @param newCapacity the new capacity (must be greater than current)
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 */
EXTERNAL void fiftyoneDegreesWeightedItemListExtend(
	fiftyoneDegreesWeightedItemList *list,
	uint32_t newCapacity,
	fiftyoneDegreesException *exception);

/**
 * Adds a copy of the item to the list, extending capacity if needed based
 * on the load factor.
 * @param list pointer to the list to add to
 * @param item pointer to the item to copy into the list
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 */
EXTERNAL void fiftyoneDegreesWeightedItemListAdd(
	fiftyoneDegreesWeightedItemList *list,
	const fiftyoneDegreesWeightedItem *item,
	fiftyoneDegreesException *exception);

/**
 * @}
 */

#endif
