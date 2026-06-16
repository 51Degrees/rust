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

#include "list.h"
#include "fiftyone.h"

fiftyoneDegreesList* fiftyoneDegreesListInit(
	fiftyoneDegreesList *list, 
	uint32_t capacity) {
	list->items = (Item*)Malloc(capacity * sizeof(Item));
	if (list->items == NULL) {
		return NULL;
	}
	list->capacity = capacity;
	list->count = 0;
	return list;
}

void fiftyoneDegreesListAdd(
	fiftyoneDegreesList *list, 
	fiftyoneDegreesCollectionItem *item) {
	assert(list->count < list->capacity);
	assert(item->collection != NULL);
	list->items[list->count++] = *item;
}

fiftyoneDegreesString* fiftyoneDegreesListGetAsString(
	fiftyoneDegreesList *list, 
	int index) {
	return (String*)list->items[index].data.ptr;
}

void fiftyoneDegreesListReset(fiftyoneDegreesList *list) {
	list->capacity = 0;
	list->count = 0;
	list->items = NULL;
}

void fiftyoneDegreesListRelease(fiftyoneDegreesList *list) {
	uint32_t i;
	for (i = 0; i < list->count; i++) {
		COLLECTION_RELEASE(list->items[i].collection, &list->items[i]);
	}
	list->count = 0;
}

void fiftyoneDegreesListFree(fiftyoneDegreesList *list) {
	fiftyoneDegreesListRelease(list);
	if (list->items != NULL) {
		Free(list->items);
	}
	list->items = NULL;
	list->capacity = 0;
}