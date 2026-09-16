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

#include "property.h"

#include "collectionKeyTypes.h"
#include "fiftyone.h"

MAP_TYPE(Collection)

#ifndef FIFTYONE_DEGREES_GET_STRING_DEFINED
#define FIFTYONE_DEGREES_GET_STRING_DEFINED
static const fiftyoneDegreesString* getString(
	const Collection *stringsCollection,
	uint32_t offset,
	Item *item,
	Exception *exception) {
	return &StoredBinaryValueGet(
		stringsCollection,
		offset,
		FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING, // metadata are strings
		item,
		exception)->stringValue;
}
#endif

const fiftyoneDegreesString* fiftyoneDegreesPropertyGetName(
	const fiftyoneDegreesCollection *stringsCollection,
	const fiftyoneDegreesProperty *property,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesException *exception) {
	return getString(
		stringsCollection,
		property->nameOffset,
		item,
		exception);
}

static int comparePropertyTypeRecordByName(
	void *state,
	Item *item,
	const CollectionKey key,
	Exception *exception) {
#	ifdef _MSC_VER
	UNREFERENCED_PARAMETER(key);
	UNREFERENCED_PARAMETER(exception);
#	endif
	const uint32_t searchNameOffset = *(uint32_t*)state;
	const PropertyTypeRecord * const nextRecord = (PropertyTypeRecord*)item->data.ptr;
	const long long result = (long long)nextRecord->nameOffset - (long long)searchNameOffset;
	return !result ? 0 : (result < 0 ? -1 : 1);
}

PropertyValueType fiftyoneDegreesPropertyGetStoredType(
	const fiftyoneDegreesCollection * const propertyTypesCollection,
	const Property * const property,
	Exception * const exception) {

	PropertyValueType result = FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING; // overwritten later

	Item item;
	DataReset(&item.data);
	bool found = false;
	for (uint32_t i = 0, n = CollectionGetCount(propertyTypesCollection);
		(!found) && (i < n);
		i++) {

		const CollectionKey propertyRecordKey = {
			i,
			CollectionKeyType_PropertyTypeRecord,
		};
		const PropertyTypeRecord * const record = (const PropertyTypeRecord*)(
			propertyTypesCollection->get(
				propertyTypesCollection,
				&propertyRecordKey,
				&item,
				exception));
		if (record != NULL && EXCEPTION_OKAY) {
			if (record->nameOffset == property->nameOffset) {
				result = record->storedValueType;
				found = true;
			}
			COLLECTION_RELEASE(propertyTypesCollection, &item);
		} else {
			break;
		}
	}
	return result;
}

PropertyValueType fiftyoneDegreesPropertyGetStoredTypeByIndex(
	const fiftyoneDegreesCollection * const propertyTypesCollection,
	const uint32_t propertyOffset,
	Exception * const exception) {

	PropertyValueType result = FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING; // overwritten later

	Item item;
	DataReset(&item.data);
	const CollectionKey recordKey = {
		propertyOffset,
		CollectionKeyType_PropertyTypeRecord,
	};
	const PropertyTypeRecord * const record = (PropertyTypeRecord*)propertyTypesCollection->get(
		propertyTypesCollection,
		&recordKey,
		&item,
		exception);
	if (EXCEPTION_OKAY) {
		result = record->storedValueType;
		COLLECTION_RELEASE(propertyTypesCollection, &item);
	}
	return result;
}

const fiftyoneDegreesString* fiftyoneDegreesPropertyGetDescription(
	const fiftyoneDegreesCollection *stringsCollection,
	const fiftyoneDegreesProperty *property,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesException *exception) {
	return getString(
		stringsCollection, 
		property->descriptionOffset,
		item, 
		exception);
}

const fiftyoneDegreesString* fiftyoneDegreesPropertyGetCategory(
	const fiftyoneDegreesCollection *stringsCollection,
	const fiftyoneDegreesProperty *property,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesException *exception) {
	return getString(
		stringsCollection, 
		property->categoryOffset, 
		item, 
		exception);
}

const fiftyoneDegreesString* fiftyoneDegreesPropertyGetUrl(
	const fiftyoneDegreesCollection *stringsCollection,
	const fiftyoneDegreesProperty *property,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesException *exception) {
	return getString(
		stringsCollection,
		property->urlOffset, 
		item,
		exception);
}

fiftyoneDegreesProperty* fiftyoneDegreesPropertyGet(
	fiftyoneDegreesCollection *properties,
	uint32_t index,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesException *exception) {

	const CollectionKey propertyKey = {
		index,
		CollectionKeyType_Property,
	};
	return (fiftyoneDegreesProperty*)properties->get(
		properties,
		&propertyKey,
		item,
		exception);
}

const fiftyoneDegreesProperty* fiftyoneDegreesPropertyGetByName(
	fiftyoneDegreesCollection *properties,
	fiftyoneDegreesCollection *strings,
	const char *requiredPropertyName,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesException *exception) {
	Item propertyNameItem;
	const String *name;
	const Property *property = NULL;
	uint32_t i = 0;
	DataReset(&propertyNameItem.data);
	uint32_t propertiesCount = CollectionGetCount(properties);
	while (i < propertiesCount && property == NULL && EXCEPTION_OKAY) {
		
		// Get the property for this index.
		const CollectionKey propertyKey = {
			i++,
			CollectionKeyType_Property,
		};
		property = (const Property*)properties->get(
			properties, 
			&propertyKey,
			item, 
			exception);
		if (property != NULL && EXCEPTION_OKAY) {
			
			// Get the property name as a string for the property at this
			// index.
			name = PropertyGetName(
				strings,
				property,
				&propertyNameItem,
				exception);
			if (name != NULL) {

				// If the property name for this index doesn't match then
				// release the property and set the property pointer back to
				// zero.
				if (EXCEPTION_OKAY &&
					strcmp(&name->value, requiredPropertyName) != 0) {
					property = NULL;
					COLLECTION_RELEASE(properties, item);
				}

				// Release the property name as this is not needed again.
				COLLECTION_RELEASE(strings, &propertyNameItem);
			}
		}
	}
	return property;
}

byte fiftyoneDegreesPropertyGetValueType(
	fiftyoneDegreesCollection *properties,
	uint32_t index,
	fiftyoneDegreesException *exception) {
	byte result = 0;
	Item item;
	Property *property;
	DataReset(&item.data);
	const CollectionKey propertyKey = {
		index,
		CollectionKeyType_Property,
	};
	property = (Property*)properties->get(
		properties,
		&propertyKey,
		&item,
		exception);
	if (property != NULL && EXCEPTION_OKAY) {
		result = property->valueType;
		COLLECTION_RELEASE(properties, &item);
	}
	return result;
}