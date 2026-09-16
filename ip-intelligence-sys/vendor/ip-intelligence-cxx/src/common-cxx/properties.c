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

#include "properties.h"
#include "fiftyone.h"

/* Array of valid property name separators */
static char separators[] = { '|', ',', ' ', '\0' };

PropertiesRequired PropertiesDefault = {
	NULL, // No array of properties
	0, // No required properties
	NULL, // No string with properties
	NULL // No list
};

typedef struct properties_source_t {
	void *state; /* State for the get method. Usually a data set */
	PropertiesGetMethod getName; /* Gets a property as a string from the
								 source, setting the collection item
								 containing the string */
	EvidencePropertiesGetMethod getEvidence; /* Populates the evidence
											 properties */
} propertiesSource;

typedef void(*matchedPropertyMethod)(PropertiesAvailable*, uint32_t);

static size_t getMemorySize(uint32_t count) {
	return sizeof(PropertiesAvailable) + (sizeof(PropertyAvailable) * count);
}

static uint32_t countAvailableProperties(propertiesSource *source) {
	Item stringItem;
	uint32_t i = 0;
	DataReset(&stringItem.data);
	while (source->getName(source->state, i, &stringItem) != NULL) {
		COLLECTION_RELEASE(stringItem.collection, &stringItem);
		i++;
	}
	return i;
}

static PropertiesAvailable* initRequiredPropertiesMemory(uint32_t count) {
	uint32_t i;
	PropertiesAvailable *available = (PropertiesAvailable*)Malloc(
		getMemorySize(count));
	if (available != NULL) {
		available->count = 0;
		available->capacity = count;
		available->items = (PropertyAvailable*)(available + 1);
		for (i = 0; i < available->capacity; i++) {
			// Initialize the evidence properties to prevent them from being
			// freed in the case that they are never allocated.
			available->items[i].evidenceProperties = NULL;
			// Initialize the delay execution.
			available->items[i].delayExecution = false;
		}
	}
	return available;
}

#ifdef _MSC_VER
#pragma warning (push)
#pragma warning (disable: 4100)
#endif
static void increaseRequiredPropertiesCount(
	PropertiesAvailable *available,
	uint32_t propertyIndex) {
	available->count++;
}
#ifdef _MSC_VER
#pragma warning (pop)
#endif

static bool resultsContain(
	PropertiesAvailable *available,
	uint32_t propertyIndex) {
	uint32_t i;
	for (i = 0; i < available->count; i++) {
		if (available->items[i].propertyIndex == propertyIndex) {
			return true;
		}
	}
	return false;
}

static void addRequiredProperty(
	PropertiesAvailable *available,
	uint32_t propertyIndex) {
	if (resultsContain(available, propertyIndex) == false) {
		available->items[available->count++].propertyIndex = propertyIndex;
	}
}

static int getPropertyIndex(
	propertiesSource *source,
	const char *requiredPropertyName,
	int requiredPropertyLength) {
	const String *test;
	Item stringItem;
	uint32_t i = 0;
	DataReset(&stringItem.data);
	test = source->getName(source->state, i, &stringItem);
	while (test != NULL) {
		if (test->size - 1 == requiredPropertyLength &&
			test != NULL &&
			_strnicmp(&test->value,
				requiredPropertyName,
				requiredPropertyLength) == 0) {
			COLLECTION_RELEASE(stringItem.collection, &stringItem);
			return i;
		}
		COLLECTION_RELEASE(stringItem.collection, &stringItem);
		test = source->getName(source->state, ++i, &stringItem);
	}
	return -1;
}

static void iteratePropertiesFromExisting(
	propertiesSource *source,
	PropertiesAvailable *available,
	PropertiesAvailable *existing,
	matchedPropertyMethod match) {
	uint32_t i;
	String *propertyName;
	int propertyIndex;
	for (i = 0; i < existing->count; i++) {
		propertyName = (String*)existing->items[i].name.data.ptr;
		if (propertyName != NULL) {
			propertyIndex = getPropertyIndex(
				source,
				&propertyName->value,
				propertyName->size - 1);
			if (propertyIndex >= 0) {
				match(available, propertyIndex);
			}
		}
	}
}

static void iteratePropertiesFromString(
	propertiesSource *source,
	PropertiesAvailable *available,
	const char* properties,
	matchedPropertyMethod match) {
	int propertyIndex;
	char *property = (char*)properties;
	const char *end = properties - 1;
	do {
		end++;
		if (strchr(separators, *end) != NULL) {
			// If the property name is one that is valid in the data set then
			// use the callback matchedProperty to provide the index.
			propertyIndex = getPropertyIndex(
				source,
				property,
				(int)(end - property));
			if (propertyIndex >= 0) {
				match(available, propertyIndex);
			}
			property = (char*)end + 1;
		}
	} while (*end != '\0');
}

static void iteratePropertiesFromArray(
	propertiesSource *source,
	PropertiesAvailable *available,
	const char** properties,
	int count,
	matchedPropertyMethod match) {
	int i, propertyIndex;
	for (i = 0; i < count; i++) {
		propertyIndex = getPropertyIndex(
			source,
			properties[i],
			(int)strlen(properties[i]));
		if (propertyIndex >= 0) {
			match(available, propertyIndex);
		}
	}
}

static uint32_t countPropertiesFromString(
	propertiesSource *source,
	const char *properties) {
	PropertiesAvailable counter;
	counter.count = 0;
	iteratePropertiesFromString(
		source,
		&counter,
		properties,
		increaseRequiredPropertiesCount);
	return counter.count;
}

static uint32_t countPropertiesFromArray(
	propertiesSource *source,
	const char **properties,
	int count) {
	PropertiesAvailable counter;
	counter.count = 0;
	iteratePropertiesFromArray(
		source,
		&counter,
		properties,
		count,
		increaseRequiredPropertiesCount);
	return counter.count;
}

static uint32_t countPropertiesFromExisting(
	propertiesSource *source,
	PropertiesAvailable *properties) {
	PropertiesAvailable counter;
	counter.count = 0;
	iteratePropertiesFromExisting(
		source,
		&counter,
		properties,
		increaseRequiredPropertiesCount);
	return counter.count;
}

static PropertiesAvailable* initRequiredPropertiesFromString(
	propertiesSource *source,
	const char* properties) {
	PropertiesAvailable *available;
	uint32_t count = countPropertiesFromString(source, properties);
	if (count == 0) {
		return NULL;
	}
	available = initRequiredPropertiesMemory(count);
	if (available != NULL) {
		iteratePropertiesFromString(
			source,
			available,
			properties,
			addRequiredProperty);
	}
	return available;
}

static PropertiesAvailable* initSpecificPropertiesFromExisting(
	propertiesSource *source,
	PropertiesAvailable *properties) {
	uint32_t count = countPropertiesFromExisting(source, properties);
	PropertiesAvailable *available = initRequiredPropertiesMemory(count);
	if (available != NULL) {
		iteratePropertiesFromExisting(
			source,
			available,
			properties,
			addRequiredProperty);
	}
	return available;
}

static PropertiesAvailable* initSpecificPropertiesFromArray(
	propertiesSource *source,
	const char** properties,
	int propertyCount) {
	uint32_t count = countPropertiesFromArray(source, properties, propertyCount);
	PropertiesAvailable *available = initRequiredPropertiesMemory(count);
	if (available != NULL) {
		iteratePropertiesFromArray(
			source,
			available,
			properties,
			propertyCount,
			addRequiredProperty);
	}
	return available;
}

static PropertiesAvailable* initAllProperties(
	propertiesSource *source) {
	uint32_t i;
	uint32_t count = countAvailableProperties(source);
	PropertiesAvailable *available = initRequiredPropertiesMemory(count);
	if (available != NULL) {
		for (i = 0; i < count; i++) {
			available->items[i].propertyIndex = i;
		}
		available->count = count;
	}
	return available;
}

static void setPropertyNames(
	propertiesSource* source,
	PropertiesAvailable* available) {
	uint32_t i;
	for (i = 0; i < available->count; i++) {
		DataReset(&available->items[i].name.data);
		source->getName(
			source->state,
			available->items[i].propertyIndex,
			&available->items[i].name);
	}
}

static void setEvidenceProperties(
	propertiesSource* source,
	PropertiesAvailable* available) {
	uint32_t i, count;
	for (i = 0; i < available->count; i++) {
		// Get the count before populating.
		count = source->getEvidence(
			source->state,
			&available->items[i],
			NULL);
		// Allocate an array big enough to be
		// populated.
		FIFTYONE_DEGREES_ARRAY_CREATE(
			fiftyoneDegreesEvidencePropertyIndex,
			available->items[i].evidenceProperties,
			count);
		if (available->items[i].evidenceProperties != NULL) {
			// Now populate the array and set the counjt.
			count = source->getEvidence(
				source->state,
				&available->items[i],
				available->items[i].evidenceProperties);
			available->items[i].evidenceProperties->count = count;
		}
	}
}

static int comparePropertyNamesAscending(const void *a, const void *b) {
	PropertyAvailable *ai = (PropertyAvailable*)a;
	PropertyAvailable *bi = (PropertyAvailable*)b;
	const char *as = STRING(ai->name.data.ptr); // name is string
	const char *bs = STRING(bi->name.data.ptr); // name is string
	assert(as != NULL && bs != NULL);
	return _stricmp(as, bs);
}

static void sortRequiredProperties(
	PropertiesAvailable *available) {
	qsort(
		(void*)available->items,
		available->count,
		sizeof(PropertyAvailable),
		comparePropertyNamesAscending);
}

static void initRequiredProperties(
	propertiesSource *source,
	PropertiesAvailable *available) {
	uint32_t i;
	String *string;

	// Set the names for each of the properties.
	setPropertyNames(source, available);

	// Set the evidence properties for each of the properties.
	setEvidenceProperties(source, available);

	// Sort these names in ascending order.
	sortRequiredProperties(available);

	// The property indexes are now invalid so need to be reset from the names.
	for (i = 0; i < available->count; i++) {
		string = (String*)available->items[i].name.data.ptr;
		if (string != NULL) {
			available->items[i].propertyIndex = getPropertyIndex(
				source,
				&string->value,
				string->size - 1);
		}
	}
}

static int comparePropertyNamesAscendingSearch(const void *a, const void *b) {
	char *as = (char*)a;
	char *bs = &((String*)((PropertyAvailable*)b)->name.data.ptr)->value;
	return StringCompare(as, bs);
}

fiftyoneDegreesPropertiesAvailable* fiftyoneDegreesPropertiesCreate(
	fiftyoneDegreesPropertiesRequired *properties,
	void *state,
	fiftyoneDegreesPropertiesGetMethod getPropertyMethod,
	fiftyoneDegreesEvidencePropertiesGetMethod getEvidencePropertiesMethod) {
	propertiesSource source;
	source.state = state;
	source.getName = getPropertyMethod;
	source.getEvidence = getEvidencePropertiesMethod;
	PropertiesAvailable *available = NULL;
	if (properties != NULL) {
		if (properties->existing != NULL) {
			// Use an existing list of properties.
			available = initSpecificPropertiesFromExisting(
				&source,
				properties->existing);
		}
		else if (properties->array != NULL && properties->count > 0) {
			// Set the required properties from the array.
			available = initSpecificPropertiesFromArray(
				&source,
				properties->array,
				properties->count);
		}
		else if (properties->string != NULL &&
			strlen(properties->string) > 0) {
			// Set the required properties from the comma separated string.
			available = initRequiredPropertiesFromString(
				&source,
				properties->string);
		}
		else {
			// Set all the properties as required properties.
			available = initAllProperties(&source);
		}
	}
	else {
		// Set all the properties as required properties.
		available = initAllProperties(&source);
	}

	// Set the require property name strings to match the require property
	// index.
	if (available != NULL) {
		initRequiredProperties(&source, available);
	}

	return available;
}

int fiftyoneDegreesPropertiesGetPropertyIndexFromRequiredIndex(
	fiftyoneDegreesPropertiesAvailable *available,
	int requiredPropertyIndex) {
	if (requiredPropertyIndex >= 0 &&
		requiredPropertyIndex < (int)available->count) {
		return available->items[requiredPropertyIndex].propertyIndex;
	}
	return -1;
}

int fiftyoneDegreesPropertiesGetRequiredPropertyIndexFromName(
	fiftyoneDegreesPropertiesAvailable *available,
	const char *propertyName) {
	int requiredPropertyIndex;
	PropertyAvailable *found = (PropertyAvailable*)
		bsearch(
			propertyName,
			available->items,
			available->count,
			sizeof(PropertyAvailable),
			comparePropertyNamesAscendingSearch);
	if (found == NULL) {
		requiredPropertyIndex = -1;
	}
	else {
		requiredPropertyIndex = (int)(found - available->items);
		assert(requiredPropertyIndex >= 0);
		assert(requiredPropertyIndex < (int)available->count);
	}
	return requiredPropertyIndex;
}

int fiftyoneDegreesPropertiesGetPropertyIndexFromName(
	fiftyoneDegreesPropertiesAvailable *available,
	const char *propertyName) {
	int requiredPropertyIndex =
		PropertiesGetRequiredPropertyIndexFromName(
			available,
			propertyName);
	if (requiredPropertyIndex >= 0) {
		return PropertiesGetPropertyIndexFromRequiredIndex(
			available,
			requiredPropertyIndex);
	}
	return -1;
}

fiftyoneDegreesString* fiftyoneDegreesPropertiesGetNameFromRequiredIndex(
	fiftyoneDegreesPropertiesAvailable *available,
	int requiredPropertyIndex) {
	return (String*)available->items[requiredPropertyIndex].name.data.ptr;
}

bool fiftyoneDegreesPropertiesIsSetHeaderAvailable(
	fiftyoneDegreesPropertiesAvailable* available) {
	const char* string;
	for (uint32_t i = 0; i < available->count; i++) {
		string = FIFTYONE_DEGREES_STRING(available->items[i].name.data.ptr); // name is string
		if (StringSubString(string, "SetHeader") == string) {
			return true;
		}
	}
	return false;
}

void fiftyoneDegreesPropertiesFree(
	fiftyoneDegreesPropertiesAvailable *available) {
	uint32_t i;
	if (available != NULL) {
		for (i = 0; i < available->count; i++) {
			if (available->items[i].name.data.ptr != NULL) {
				COLLECTION_RELEASE(available->items[i].name.collection,
					&available->items[i].name);
			}
			if (available->items[i].evidenceProperties != NULL) {
				Free(available->items[i].evidenceProperties);
			}
		}
		Free(available);
	}
}