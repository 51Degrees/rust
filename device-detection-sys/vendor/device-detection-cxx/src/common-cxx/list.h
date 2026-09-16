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

#ifndef FIFTYONE_DEGREES_LIST_H_INCLUDED
#define FIFTYONE_DEGREES_LIST_H_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesList List
 *
 * A more manageable way to store arrays.
 *
 * ## Introduction
 *
 * Lists are a way of storing an array of elements in an easy to manage way.
 * This means that the capacity, and the number of elements are both known
 * quantities to any method handling the list, in contrast to a pure array
 * where neither are known.
 * Lists hold collection items, so when they are finished with, they must be
 * freed correctly using the #fiftyoneDegreesListFree method.
 * Cleanup is also handled so as to be simpler to use than an array. By freeing
 * the list, all elements are released from their collection and the memory
 * for the list is freed.
 *
 * ## Creation
 *
 * Lists are created by allocating memory to the list structure, then calling
 * the init method #fiftyoneDegreesListInit to allocate memory for list items
 * and initialise the count and capacity.
 *
 * ## Add
 *
 * Collection items are added to a list using the #fiftyoneDegreesListAdd
 * method. This then holds the reference to the collection item until the list
 * is freed.
 *
 * ## Get
 *
 * Items can be fetched from the list by accessing the collection item
 * directly:
 * ```
 * list->items[index]
 * ```
 * then casting the pointer it contains to the correct type. Or for strings
 * the #fiftyoneDegreesListGetAsString method can be used to access and cast
 * in one call.
 *
 * ## Free
 *
 * Lists must be freed using the #fiftyoneDegreesListFree method. This releases
 * all collection items stored within, and frees the memory for the internal
 * array which stores their pointers.
 *
 * ## Reuse
 *
 * Lists can be reused by releasing all the collection items with the
 * #fiftyoneDegreesListRelease method, clearing the list ready for a new list
 * of items to be added.
 *
 * ## Usage Example
 *
 * ```
 * fiftyoneDegreesList list;
 * fiftyoneDegreesCollectionItem *item;
 *
 * // Initialise the list with the capacity for 1 entry
 * fiftyoneDegreesListInit(&list, 1);
 *
 * // Add an item from a collection to the list
 * fiftyoneDegreesListAdd(&list, item);
 *
 * // Get the first item in the list as a string
 * fiftyoneDegreesString *string = fiftyoneDegreesListGetAsString(&list, 0);
 *
 * // Do Something with the string
 * // ...
 *
 * // Free the list, releasing the items it contains back to their respective
 * // collections
 * fiftyoneDegreesListFree(&list);
 * ```
 *
 * @{
 */

#include "collection.h"
#include "string.h"
#include "common.h"

/**
 * List structure which contains a list of collection items.
 */
typedef struct fiftyone_degrees_list_t {
	fiftyoneDegreesCollectionItem *items; /**< Array of items */
	uint32_t capacity; /**< Capacity of the list to store items */
	uint32_t count; /**< Number of items currently in the list  */
} fiftyoneDegreesList;

/**
 * Initialise the list by allocating space for the items in the list structure.
 * @param list pointer to list structure to initialise
 * @param capacity number of items expected in the list
 * @return the initialised list or NULL if memory could not be allocated
 */
EXTERNAL fiftyoneDegreesList* fiftyoneDegreesListInit(
	fiftyoneDegreesList *list,
	uint32_t capacity);

/**
 * Adds a collection item to a list. The reference to the item will be released
 * when the list is released or freed.
 * @param list to add the item to
 * @param item to add to the list
 */
EXTERNAL void fiftyoneDegreesListAdd(
	fiftyoneDegreesList *list,
	fiftyoneDegreesCollectionItem *item);

/**
 * Gets the item at the index provided as a string.
 * @param list to get the item from
 * @param index of the item in the list
 * @return the requested list item as a string
 */
EXTERNAL fiftyoneDegreesString* fiftyoneDegreesListGetAsString(
	fiftyoneDegreesList *list,
	int index);

/**
 * Frees the memory allocated to the list structure and release all items
 * stored in it.
 * @param list to free
 */
EXTERNAL void fiftyoneDegreesListFree(fiftyoneDegreesList *list);

/**
 * Resets a newly allocated list to a clean empty state.
 * @param list to reset
 */
EXTERNAL void fiftyoneDegreesListReset(fiftyoneDegreesList *list);

/**
 * Releases all the items stored in the list.
 * @param list to release
 */
EXTERNAL void fiftyoneDegreesListRelease(fiftyoneDegreesList *list);

/**
 * @}
 */

#endif