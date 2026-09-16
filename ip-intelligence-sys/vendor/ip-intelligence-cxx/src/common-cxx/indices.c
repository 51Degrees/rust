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

#include "indices.h"

#include "collectionKeyTypes.h"
#include "fiftyone.h"

// Working data structure used to construct the index.
typedef struct map_t {
	uint32_t availableProperty; // available property index
	int16_t propertyIndex; // index in the properties collection
} map;

// Gets the index of the profile id in the property profile index.
static uint32_t getProfileIdIndex(
	IndicesPropertyProfile* index, 
	uint32_t profileId) {
	return profileId - index->minProfileId;
}

// Loops through the values associated with the profile setting the index at 
// the position for the property and profile to the first value index from the
// profile.
static void addProfileValuesMethod(
	IndicesPropertyProfile* index, // index in use or null if not available
	map* propertyIndexes, // property indexes in ascending order
	fiftyoneDegreesCollection* values, // collection of values
	Profile* profile, 
	Exception* exception) {
#ifdef FIFTYONE_DEGREES_REDUCED_FILE
	// A reduced size data file does not contain profile ids, so this method
	// cannot be implemented.
#ifdef _MSC_VER
	UNREFERENCED_PARAMETER(index);
	UNREFERENCED_PARAMETER(propertyIndexes);
	UNREFERENCED_PARAMETER(values);
	UNREFERENCED_PARAMETER(profile);
#endif
	EXCEPTION_SET(NOT_IMPLEMENTED);
#else
	uint32_t valueIndex;
	Item valueItem; // The current value memory
	Value* value; // The current value pointer
	DataReset(&valueItem.data);
	
	uint32_t* first = (uint32_t*)(profile + 1); // First value for the profile
	uint32_t base = getProfileIdIndex(index, profile->profileId) * 
		index->availablePropertyCount;

	CollectionKey valueKey = {
		0,
		CollectionKeyType_Value,
	};
	// For each of the values associated with the profile check to see if it
	// relates to a new property index. If it does then record the first value
	// index and advance the current index to the next pointer.
	for (uint32_t i = 0, p = 0;
		i < profile->valueCount &&
		p < index->availablePropertyCount &&
		EXCEPTION_OKAY;
		i++) {
		valueKey.indexOrOffset.offset = *(first + i);
		value = values->get(values, &valueKey, &valueItem, exception);
		if (value != NULL && EXCEPTION_OKAY) {

			// If the value doesn't relate to the next property index then 
			// move to the next property index.
			while (p < index->availablePropertyCount && // first check validity 
				// of the subscript and then use it
                propertyIndexes[p].propertyIndex < value->propertyIndex) {
				p++;
			}

			// If the value relates to the next property index being sought 
			// then record the first value in the profile associated with the
			// property.
			if (p < index->availablePropertyCount &&
				value->propertyIndex == propertyIndexes[p].propertyIndex) {
				valueIndex = base + propertyIndexes[p].availableProperty;
				index->valueIndexes[valueIndex] = i;
				p++;
				index->filled++;
			}
			COLLECTION_RELEASE(values, &valueItem);
		}
	}
#endif
}

static void iterateProfiles(
	fiftyoneDegreesCollection* profiles,
	fiftyoneDegreesCollection* profileOffsets,
	IndicesPropertyProfile* index, // index in use or null if not available
	map* propertyIndexes, // property indexes in ascending order
	fiftyoneDegreesCollection* values, // collection of values
	Exception *exception) {
	Profile* profile; // The current profile pointer
	Item profileItem; // The current profile memory
	ProfileOffset* profileOffset; // The current profile offset pointer
	Item profileOffsetItem; // The current profile offset memory
	DataReset(&profileItem.data);
	DataReset(&profileOffsetItem.data);
	CollectionKey profileOffsetKey = {
		0,
		CollectionKeyType_ProfileOffset,
	};
	CollectionKey profileKey = {
		0,
		CollectionKeyType_Profile,
	};
	for (uint32_t i = 0; 
		i < index->profileCount && EXCEPTION_OKAY;
		i++) {
		profileOffsetKey.indexOrOffset.offset = i;
		profileOffset = profileOffsets->get(
			profileOffsets,
			&profileOffsetKey,
			&profileOffsetItem,
			exception);
		if (profileOffset != NULL && EXCEPTION_OKAY) {
			profileKey.indexOrOffset.offset = profileOffset->offset;
			profile = profiles->get(
				profiles,
				&profileKey,
				&profileItem,
				exception);
			if (profile != NULL && EXCEPTION_OKAY) {
				addProfileValuesMethod(
					index,
					propertyIndexes,
					values,
					profile,
					exception);
				COLLECTION_RELEASE(profiles, &profileItem);
			}
			COLLECTION_RELEASE(profileOffsets, &profileOffsetItem);
		}
	}
}

// As the profileOffsets collection is ordered in ascending profile id the 
// first and last entries are the min and max available profile ids.
static uint32_t getProfileId(
	fiftyoneDegreesCollection* profileOffsets,
	uint32_t index,
	Exception* exception) {
#ifdef FIFTYONE_DEGREES_REDUCED_FILE
	// A reduced size data file does not contain profile ids, so this method
	// cannot be implemented.
#ifdef _MSC_VER
	UNREFERENCED_PARAMETER(profileOffsets);
	UNREFERENCED_PARAMETER(index);
#endif
	EXCEPTION_SET(NOT_IMPLEMENTED);
	return 0;
#else
	uint32_t profileId = 0;
	ProfileOffset* profileOffset; // The profile offset pointer
	Item profileOffsetItem; // The profile offset memory
	DataReset(&profileOffsetItem.data);
	const CollectionKey profileOffsetKey = {
		index,
		CollectionKeyType_ProfileOffset,
	};
	profileOffset = profileOffsets->get(
		profileOffsets,
		&profileOffsetKey,
		&profileOffsetItem,
		exception);
	if (profileOffset != NULL && EXCEPTION_OKAY) {
		profileId = profileOffset->profileId;
		COLLECTION_RELEASE(profileOffsets, &profileOffsetItem);
	}
	return profileId;
#endif
}

static int comparePropertyIndexes(const void* a, const void* b) {
	return ((map*)a)->propertyIndex - ((map*)b)->propertyIndex;
}

// Build an ascending ordered array of the property indexes.
static map* createPropertyIndexes(
	PropertiesAvailable* available,
	Exception* exception) {
	map* index = (map*)Malloc(sizeof(map) * available->count);
	if (index == NULL) {
		EXCEPTION_SET(FIFTYONE_DEGREES_STATUS_INSUFFICIENT_MEMORY);
		return NULL;
	}
	for (uint32_t i = 0; i < available->count; i++) {
		index[i].availableProperty = i;
		index[i].propertyIndex = (int16_t)available->items[i].propertyIndex;
	}
	qsort(index, available->count, sizeof(map*), comparePropertyIndexes);
	return index;
}

fiftyoneDegreesIndicesPropertyProfile*
fiftyoneDegreesIndicesPropertyProfileCreate(
	fiftyoneDegreesCollection* profiles,
	fiftyoneDegreesCollection* profileOffsets,
	fiftyoneDegreesPropertiesAvailable* available,
	fiftyoneDegreesCollection* values,
	fiftyoneDegreesException* exception) {

	// Create the ordered list of property indexes.
	map* propertyIndexes = createPropertyIndexes(available, exception);
	if (propertyIndexes == NULL) {
		return NULL;
	}

	// Allocate memory for the index and set the fields.
	IndicesPropertyProfile* index = (IndicesPropertyProfile*)Malloc(
		sizeof(IndicesPropertyProfile));
	if (index == NULL) {
		EXCEPTION_SET(FIFTYONE_DEGREES_STATUS_INSUFFICIENT_MEMORY);
		return NULL;
	}
	index->filled = 0;
	index->profileCount = CollectionGetCount(profileOffsets);
	index->minProfileId = getProfileId(profileOffsets, 0, exception);
	if (!EXCEPTION_OKAY) {
		Free(index);
		Free(propertyIndexes);
		return NULL;
	}
	index->maxProfileId = getProfileId(
		profileOffsets,
		index->profileCount - 1,
		exception);
	if (!EXCEPTION_OKAY) {
		Free(index);
		Free(propertyIndexes);
		return NULL;
	}
	index->availablePropertyCount = available->count;
	index->size = (index->maxProfileId - index->minProfileId + 1) * 
		available->count;
	
	// Allocate memory for the values index and set the fields.
	index->valueIndexes =(uint32_t*)Malloc(sizeof(uint32_t) * index->size);
	if (index->valueIndexes == NULL) {
		EXCEPTION_SET(FIFTYONE_DEGREES_STATUS_INSUFFICIENT_MEMORY);
		Free(index);
		Free(propertyIndexes);
		return NULL;
	}

	// For each of the profiles in the collection call add the property value
	// indexes to the index array.
	iterateProfiles(
		profiles, 
		profileOffsets, 
		index, 
		propertyIndexes,
		values,
		exception);
	Free(propertyIndexes);

	// Return the index or free the memory if there was an exception.
	if (EXCEPTION_OKAY) {
		return index;
	}
	else {
		Free(index->valueIndexes);
		Free(index);
		return NULL;
	}
}

void fiftyoneDegreesIndicesPropertyProfileFree(
	fiftyoneDegreesIndicesPropertyProfile* index) {
	Free(index->valueIndexes);
	Free(index);
}

uint32_t fiftyoneDegreesIndicesPropertyProfileLookup(
	fiftyoneDegreesIndicesPropertyProfile* index,
	uint32_t profileId,
	uint32_t availablePropertyIndex) {
	uint32_t valueIndex = 
		(getProfileIdIndex(index, profileId) * index->availablePropertyCount) + 
		availablePropertyIndex;
	assert(valueIndex < index->size);
	return index->valueIndexes[valueIndex];
}
