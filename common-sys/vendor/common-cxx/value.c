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

#include "value.h"

#include "collectionKeyTypes.h"
#include "fiftyone.h"

MAP_TYPE(Value);
MAP_TYPE(Collection);
MAP_TYPE(CollectionItem);

typedef struct value_search_t {
	const Collection *strings;
	const char *valueName;
	PropertyValueType valueType;
	StringBuilder *tempBuilder;
} valueSearch;

static int compareValueByName(
	void *state,
	Item *item,
	CollectionKey key,
	Exception *exception) {
#	ifdef _MSC_VER
	UNREFERENCED_PARAMETER(key);
#	endif
	int result = 0;
	Item name;
	const StoredBinaryValue *value;
	valueSearch *search = (valueSearch*)state;
	DataReset(&name.data);
	if (search->tempBuilder) {
		StringBuilderInit(search->tempBuilder);
	}
	value = ValueGetContent(
		search->strings,
		(Value*)item->data.ptr,
		search->valueType,
		&name,
		exception);
	if (value != NULL && EXCEPTION_OKAY) {
		result = StoredBinaryValueCompareWithString(
			value,
			search->valueType,
			search->valueName,
			search->tempBuilder,
			exception);
		COLLECTION_RELEASE(search->strings, &name);
	}
	return result;
}

const StoredBinaryValue* fiftyoneDegreesValueGetContent(
	const Collection *strings,
	const Value *value,
	PropertyValueType storedValueType,
	CollectionItem *item,
	Exception *exception) {

	return StoredBinaryValueGet(strings, value->nameOffset, storedValueType, item, exception);
}

const String* fiftyoneDegreesValueGetName(
	const Collection *strings,
	const Value *value,
	CollectionItem *item,
	Exception *exception) {
	return &StoredBinaryValueGet(
		strings,
		value->nameOffset,
		FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING, // legacy contract
		item,
		exception)->stringValue;
}

const String* fiftyoneDegreesValueGetDescription(
	const Collection *strings,
	const Value *value,
	CollectionItem *item,
	Exception *exception) {
#ifdef FIFTYONE_DEGREES_REDUCED_FILE
	// A reduced size data file does not contain value descriptions, so this
	// method cannot be implemented.
#ifdef _MSC_VER
	UNREFERENCED_PARAMETER(strings);
	UNREFERENCED_PARAMETER(value);
	UNREFERENCED_PARAMETER(item);
#endif
	EXCEPTION_SET(NOT_IMPLEMENTED)
	return NULL;
#else
	return &StoredBinaryValueGet(
		strings,
		value->descriptionOffset,
		FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING, // description is string
		item,
		exception)->stringValue;
#endif
}

const String* fiftyoneDegreesValueGetUrl(
	const Collection *strings,
	const Value *value,
	CollectionItem *item,
	Exception *exception) {
	if (value->urlOffsetOrWeight < 0) {
		return NULL;
	}
	return &StoredBinaryValueGet(
		strings,
		value->urlOffsetOrWeight,
		FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING, // URL is string
		item,
		exception)->stringValue;
}

const Value* fiftyoneDegreesValueGet(
	const Collection *values,
	uint32_t valueIndex,
	CollectionItem *item,
	Exception *exception) {
	const CollectionKey valueKey = {
		valueIndex,
		CollectionKeyType_Value,
	};
	return (const Value*)values->get(
		values,
		&valueKey,
		item, 
		exception);
}

long fiftyoneDegreesValueGetIndexByName(
	Collection *values,
	Collection *strings,
	Property *property,
	const char *valueName,
	Exception *exception) {

	return fiftyoneDegreesValueGetIndexByNameAndType(
		values,
		strings,
		property,
		FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING, // legacy contract
		valueName,
		exception);
}

long fiftyoneDegreesValueGetIndexByNameAndType(
	const Collection *values,
	const Collection *strings,
	const Property *property,
	fiftyoneDegreesPropertyValueType storedValueType,
	const char *valueName,
	Exception *exception) {
	Item item;
	valueSearch search;
	long index;
	DataReset(&item.data);
	search.valueName = valueName;
	search.strings = strings;
	search.valueType = storedValueType;

	const bool isString = (storedValueType == FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING);
	const size_t requiredSize = strlen(valueName) + 3;
	char * const buffer = isString ? NULL : Malloc(requiredSize);
	StringBuilder tempBuilder = { buffer, requiredSize };
	search.tempBuilder = isString ? NULL : &tempBuilder;

	index = CollectionBinarySearch(
		values,
		&item,
		(CollectionIndexOrOffset){property->firstValueIndex},
		(CollectionIndexOrOffset){property->lastValueIndex},
		CollectionKeyType_Value,
		(void*)&search,
		compareValueByName,
		exception);
	if (buffer) {
		Free(buffer);
	}
	if (EXCEPTION_OKAY) {
		COLLECTION_RELEASE(values, &item);
	}
	return index;
}

const Value* fiftyoneDegreesValueGetByName(
	const Collection *values,
	const Collection *strings,
	const Property *property,
	const char *valueName,
	CollectionItem *item,
	Exception *exception) {

	return fiftyoneDegreesValueGetByNameAndType(
		values,
		strings,
		property,
		FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING, // legacy contract
		valueName,
		item,
		exception);
}

const Value* fiftyoneDegreesValueGetByNameAndType(
	const Collection * const values,
	const Collection * const strings,
	const Property * const property,
	const fiftyoneDegreesPropertyValueType storedValueType,
	const char * const valueName,
	CollectionItem * const item,
	Exception * const exception) {
	valueSearch search;
	Value *value = NULL;
	search.valueName = valueName;
	search.strings = strings;
	search.valueType = storedValueType;

	const bool isString = (storedValueType == FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING);
	const size_t requiredSize = strlen(valueName) + 3;
	char * const buffer = isString ? NULL : Malloc(requiredSize);
	StringBuilder tempBuilder = { buffer, requiredSize };
	search.tempBuilder = isString ? NULL : &tempBuilder;

	if (
		(int)property->firstValueIndex != -1 &&
		CollectionBinarySearch(
			values,
			item,
			(CollectionIndexOrOffset){property->firstValueIndex},
			(CollectionIndexOrOffset){property->lastValueIndex},
			CollectionKeyType_Value,
			(void*)&search,
			compareValueByName,
			exception) >= 0 &&
		EXCEPTION_OKAY) {
		value = (Value*)item->data.ptr;
	}
	if (buffer) {
		Free(buffer);
	}
	return value;
}

bool fiftyoneDegreesValueIsWeighted(
	const Value *value) {
	return FIFTYONE_DEGREES_VALUE_IS_MASKED(value);
}

uint16_t fiftyoneDegreesValueGetWeight(
	const Value *value) {
	if (FIFTYONE_DEGREES_VALUE_IS_MASKED(value)) {
		return (uint16_t)(value->urlOffsetOrWeight & 0xFFFF);
	}
	return 0;
}
