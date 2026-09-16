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

#include "overrides.h"
#include "fiftyone.h"

MAP_TYPE(Collection)

typedef struct override_profile_ids_state_t {
	fiftyoneDegreesOverrideProfileIdMethod callback; /* Method to call when
													 a profile ids is found */
	void *state; /* State to pass to the callback method */
} overrideProfileIdsState;

typedef struct add_state_t {
	OverridePropertyArray *properties; /* Properties available to be
									   overridden */
	OverrideValueArray *values; /* Values from evidence to use when fetching
								overridden properties */
} addState;

#ifdef _MSC_VER
#pragma warning (disable:4100)  
#endif
static void collectionRelease(Item *item) {
	// Do Nothing.
}
#ifdef _MSC_VER
#pragma warning (default:4100)  
#endif

/* Prefix to use when comparing property names. */
#define OVERRIDE_PREFIX "51D_"

/**
 * Checks if the pair (p) have a field name that matches the target (t).
 * The last byte of t is null where as fieldLength is the length of printable
 * characters. Take 1 from the t to compare length.
 */
#define IS_HEADER_MATCH(t,p) \
	(StringCompareLength(\
	skipPrefix(true, (char*)pair->item.key), t, sizeof(t)) == 0)

static const Collection dummyCollection = { 
	NULL, 
	collectionRelease, 
	NULL, 
	NULL,
	0, 
	0, 
	0,
};

/**
 * Checks if the string starts with the prefix string.
 * @return true if the prefix is present, otherwise false.
 */
static bool startsWith(String *string, String *prefix) {
	int16_t i = 0;

	// Check that the string is longer than the prefix. Also prevents the
	// prefix and the string being the same string.
	if (string->size > prefix->size) {

		// Loop checking characters until one doesn't match or all the prefix
		// characters are matched.
		while (i < prefix->size - 1) {
			if ((&string->value)[i] != (&prefix->value)[i]) {
				return false;
			}
			i++;
		}
		return true;
	}
	return false;
}

/**
 * If there is an expected prefix to be ignored then check for it in the
 * property name and skip those characters if present.
 * @param prefix true if there is a prefix to be skipped
 * @param field the name of the field provided
 * @return a pointer to the start of the field name for checking.
 */
static const char* skipPrefix(bool prefixed, const char *field) {
	char *result = (char*)field;
	if (result && prefixed == true && strlen(result) > sizeof(OVERRIDE_PREFIX) &&
		StringCompareLength(
			field,
			OVERRIDE_PREFIX,
			sizeof(OVERRIDE_PREFIX) - 1) == 0) {
		result += sizeof(OVERRIDE_PREFIX) - 1;
	}
	return result;
}

static int getRequiredPropertyIndexFromName(
	OverridePropertyArray *properties,
	const char *name) {
	uint32_t i;
	const char *current;
	OverrideProperty *property;

	// Skip the field name prefix.
	name = (const char*)skipPrefix(properties->prefix, name);

	// Search for the property name in the array of properties that can support
	// being overridden.
	for (i = 0; i < properties->count; i++) {
		property = &properties->items[i];
		current = STRING(property->available->name.data.ptr); // name is string
		if (StringCompare((char*)current, (char*)name) == 0) {
			return property->requiredPropertyIndex;
		}
	}
	return -1;
}

bool fiftyoneDegreesOverridesAdd(
	fiftyoneDegreesOverrideValueArray *values,
	int requiredPropertyIndex,
	const char *value) {
	uint32_t currentOverrideIndex = 0;
	size_t length;
	String *copy;
	OverrideValue *override;
	if (requiredPropertyIndex >= 0 && values->count < values->capacity) {

		// Set the override either as a new item, or override an existing
		// one if there is already one with the same required property
		// index.
		while (currentOverrideIndex < values->count &&
			(int)values->items[currentOverrideIndex].requiredPropertyIndex !=
			requiredPropertyIndex) {
			currentOverrideIndex++;
		}
		override = &values->items[currentOverrideIndex];

		if (currentOverrideIndex == values->count) {
			// Increment the override count and set the required property
			// index.
			values->count++;
			override->requiredPropertyIndex = requiredPropertyIndex;
		}

		// Ensure there is sufficient memory for the string being copied.
		length = strlen(value);
		copy = (String*)fiftyoneDegreesDataMalloc(
			&override->string,
			sizeof(String) + length);

		// Copy the string from the evidence pair to the override data 
		// item.
		memcpy(&copy->value, value, length + 1);
		copy->size = (int16_t)(length + 1);
	}

	return values->count < values->capacity;
}


static bool addOverrideToResults(void *state, EvidenceKeyValuePair *pair) {
	addState *add = (addState*)state;

	// Find the required property index, if any for the field.
	int requiredPropertyIndex = getRequiredPropertyIndexFromName(
		add->properties,
		pair->item.key);

	return fiftyoneDegreesOverridesAdd(
		add->values,
		requiredPropertyIndex,
		(const char*)pair->parsedValue);
}

static uint32_t countOverridableProperties(
	PropertiesAvailable *available,
	void *state,
	fiftyoneDegreesOverridesFilterMethod filter) {
	uint32_t i, count = 0;
	for (i = 0; i < available->count; i++) {
		if (filter(state, i) == true) {
			count++;
		}
	}
	return count;
}

static uint32_t addOverridableProperties(
	PropertiesAvailable *available,
	OverridePropertyArray *properties,
	void *state,
	fiftyoneDegreesOverridesFilterMethod filter) {
	uint32_t i, count = 0;
	OverrideProperty *property;
	for (i = 0; i < available->count; i++) {
		if (filter(state, i) == true) {
			property = &properties->items[properties->count++];
			property->available = &available->items[i];
			property->requiredPropertyIndex = i;
		}
	}
	return count;
}

fiftyoneDegreesOverrideValueArray* fiftyoneDegreesOverrideValuesCreate(
	uint32_t capacity) {
	uint32_t i;
	OverrideValue *item;
	fiftyoneDegreesOverrideValueArray* overrides;
	FIFTYONE_DEGREES_ARRAY_CREATE(OverrideValue, overrides, capacity);
	if (overrides != NULL) {
		for (i = 0; i < capacity; i++) {
			item = &overrides->items[i];
			item->requiredPropertyIndex = 0;
			DataReset(&item->string);
		}
	}
	return overrides;
}

fiftyoneDegreesOverridePropertyArray* fiftyoneDegreesOverridePropertiesCreate(
	fiftyoneDegreesPropertiesAvailable *available,
	bool prefix,
	void *state,
	fiftyoneDegreesOverridesFilterMethod filter) {
	OverridePropertyArray *properties = NULL;
	uint32_t count = countOverridableProperties(available, state, filter);
	if (count > 0) {
		FIFTYONE_DEGREES_ARRAY_CREATE(OverrideProperty, properties, count);
		if (properties != NULL) {
			properties->prefix = prefix;
			addOverridableProperties(available, properties, state, filter);
		}
	} 
	return properties;
}

void fiftyoneDegreesOverridePropertiesFree(
	fiftyoneDegreesOverridePropertyArray *properties) {
	Free(properties);
}

uint32_t fiftyoneDegreesOverridesExtractFromEvidence(
	fiftyoneDegreesOverridePropertyArray *properties,
	fiftyoneDegreesOverrideValueArray *values,
	fiftyoneDegreesEvidenceKeyValuePairArray *evidence) {
	addState state;
	uint32_t count = 0;
	if (properties != NULL && values != NULL) {
		state.values = values;
		state.properties = properties;
		count = EvidenceIterate(
			evidence,
			FIFTYONE_DEGREES_EVIDENCE_COOKIE |
			FIFTYONE_DEGREES_EVIDENCE_QUERY,
			&state,
			addOverrideToResults);
	}
	return count;
}

fiftyoneDegreesString* fiftyoneDegreesOverrideValuesGetFirst(
	fiftyoneDegreesOverrideValueArray *values,
	uint32_t requiredPropertyIndex,
	fiftyoneDegreesCollectionItem *item) {
	uint32_t i;
	if (values != NULL) {
		for (i = 0; i < values->count; i++) {
			if (values->items[i].requiredPropertyIndex == 
				requiredPropertyIndex) {
				item->collection = (Collection*)&dummyCollection;

				// Copy the pointer to the string in the values array. Set
				// allocated and used to false as the memory must not be
				// freed until the results the overrides are part of are 
				// freed.
				item->data.ptr = values->items[i].string.ptr;
				item->data.allocated = 0;
				item->data.used = 0;

				return (String*)item->data.ptr;
			}
		}
	}
	return NULL;
}

bool fiftyoneDegreesOverrideHasValueForRequiredPropertyIndex(
	fiftyoneDegreesOverrideValueArray *values,
	uint32_t requiredPropertyIndex) {
	uint32_t i;
	OverrideValue *current;
	if (values != NULL) {
		// Loop through the values available from the overrides checking they
		// match the required property index.
		for (i = 0; i < values->count; i++) {
			current = &values->items[i];
			if (current->requiredPropertyIndex == requiredPropertyIndex) {
				return true;
			}
		}
	}
	return false;
}

uint32_t fiftyoneDegreesOverrideValuesAdd(
	fiftyoneDegreesOverrideValueArray *values,
	uint32_t requiredPropertyIndex,
	fiftyoneDegreesList *list) {
	uint32_t i, count = 0;
	OverrideValue *current;
	Item valueItem;
	if (values != NULL && values->count > 0) {

		// Use a dummy collection so that the call to release will work
		// if the client respects the collection pattern.
		valueItem.collection = (Collection*)&dummyCollection;
		valueItem.handle = NULL;

		// Loop through the values available from the overrides checking they
		// match the required property index.
		for (i = 0; i < values->count && list->count < list->capacity; i++) {
			current = &values->items[i];
			if (current->requiredPropertyIndex == requiredPropertyIndex) {

				// Copy the pointer to the string in the values array. Set
				// allocated and used to false as the memory must not be
				// freed until the results the overrides are part of are 
				// freed.
				valueItem.data.ptr = current->string.ptr;
				valueItem.data.allocated = 0;
				valueItem.data.used = 0;

				// Add the collection item to the list of values.
				ListAdd(list, &valueItem);
				count++;
			}
		}
	}
	return count;
}

int fiftyoneDegreesOverridesGetOverridingRequiredPropertyIndex(
	fiftyoneDegreesPropertiesAvailable *available,
	uint32_t requiredPropertyIndex) {
	String *compare, *propertyName;
	uint32_t i;
	propertyName = (String*)available->items[
		requiredPropertyIndex].name.data.ptr;
	for (i = 0; i < available->count; i++) {
		if (i != requiredPropertyIndex) {
			compare = (String*)available->items[i].name.data.ptr;
			if (startsWith(compare, propertyName)) {

				return i;
			}
		}
	}
	return -1;
}

void fiftyoneDegreesOverrideValuesFree(
	fiftyoneDegreesOverrideValueArray *overrides) {
	uint32_t i;
	OverrideValue *item;
	if (overrides != NULL) {
		for (i = 0; i < overrides->capacity; i++) {
			item = &overrides->items[i];
			if (item->string.ptr != NULL && item->string.allocated > 0) {
				Free(item->string.ptr);
			}
		}
		Free(overrides);
		overrides = NULL;
	}
}

void fiftyoneDegreesOverrideValuesReset(
	fiftyoneDegreesOverrideValueArray *overrides) {
	uint32_t i;
	OverrideValue *item;
	if (overrides != NULL) {
		for (i = 0; i < overrides->capacity; i++) {
			item = &overrides->items[i];
			if (item->string.ptr != NULL && item->string.allocated > 0) {
				memset(item->string.ptr, 0, item->string.allocated);
				item->string.used = 0;
				item->requiredPropertyIndex = 0;
			}
		}
		overrides->count = 0;
	}
}

static void extractProfileId(char *value, overrideProfileIdsState *state) {
	if (*value >= 0 && isdigit(*value) != 0) {
		int profileId = atoi(value);
		if (profileId >= 0) {
			state->callback(state->state, profileId);
		}
	}
}

static void extractProfileIds(overrideProfileIdsState *state, char *value) {
	char *current = value, *previous = value;
	while (*current != '\0') {
		if (*current < 0 || isdigit(*current) == 0) {
			extractProfileId(previous, state);
			previous = current + 1;
		}
		current++;
	}
	extractProfileId(previous, state);
}

static bool iteratorProfileId(void *state, EvidenceKeyValuePair *pair) {
	if (IS_HEADER_MATCH("ProfileIds", pair)) {
		extractProfileIds(
			(overrideProfileIdsState*)state, 
			(char*)pair->parsedValue);
	}
	return true;
}

void fiftyoneDegreesOverrideProfileIds(
	fiftyoneDegreesEvidenceKeyValuePairArray *evidence,
	void *state,
	fiftyoneDegreesOverrideProfileIdMethod callback) {
	overrideProfileIdsState iterateState = { callback, state };
	EvidenceIterate(
		evidence,
		FIFTYONE_DEGREES_EVIDENCE_COOKIE |
		FIFTYONE_DEGREES_EVIDENCE_QUERY,
		&iterateState,
		iteratorProfileId);
}
