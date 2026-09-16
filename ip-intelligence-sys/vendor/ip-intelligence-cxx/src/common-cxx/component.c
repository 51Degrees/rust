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

#include "component.h"
#include "fiftyone.h"
#include "collectionKeyTypes.h"

uint32_t fiftyoneDegreesComponentGetFinalSize(
	const void *initial,
    Exception * const exception) {
#	ifdef _MSC_VER
    UNREFERENCED_PARAMETER(exception);
#	endif
	Component *component = (Component*)initial;
	int32_t trailing = (component->keyValuesCount - 1) * 
		sizeof(fiftyoneDegreesComponentKeyValuePair);
	return (uint32_t)(sizeof(Component) + trailing);
}

uint32_t fiftyoneDegreesComponentGetDefaultProfileId(
	fiftyoneDegreesCollection *profiles,
	fiftyoneDegreesComponent *component,
	fiftyoneDegreesException *exception) {
#ifdef FIFTYONE_DEGREES_REDUCED_FILE
	// A reduced size data file does not contain profile ids, so this method
	// cannot be implemented.
#ifdef _MSC_VER
	UNREFERENCED_PARAMETER(profiles);
	UNREFERENCED_PARAMETER(component);
#endif
	EXCEPTION_SET(NOT_IMPLEMENTED)
	return 0;
#else
	uint32_t profileId = 0;
	Item profileItem;
	Profile *profile;
	DataReset(&profileItem.data);
	const CollectionKey profileKey = {
		component->defaultProfileOffset,
		CollectionKeyType_Profile,
	};
	profile = (Profile*)profiles->get(
		profiles,
		&profileKey,
		&profileItem,
		exception);
	if (profile != NULL && EXCEPTION_OKAY) {
		profileId = profile->profileId;
		COLLECTION_RELEASE(profiles, &profileItem);
	}
	return profileId;
#endif
}

const fiftyoneDegreesString* fiftyoneDegreesComponentGetName(
	fiftyoneDegreesCollection *stringsCollection,
	fiftyoneDegreesComponent *component,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesException *exception) {
	return &StoredBinaryValueGet(
		stringsCollection, 
		component->nameOffset,
		FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING, // name is string
		item,
		exception)->stringValue;
}

const fiftyoneDegreesComponentKeyValuePair* 
fiftyoneDegreesComponentGetKeyValuePair(
	fiftyoneDegreesComponent *component,
	uint16_t index,
	fiftyoneDegreesException *exception) {
#ifndef FIFTYONE_DEGREES_EXCEPTIONS_DISABLED
	if (index > component->keyValuesCount) {
		EXCEPTION_SET(COLLECTION_INDEX_OUT_OF_RANGE);
		return NULL;
	}
#endif
	return &(&component->firstKeyValuePair)[index];
}

#ifndef FIFTYONE_DEGREES_MEMORY_ONLY

void* fiftyoneDegreesComponentReadFromFile(
	const fiftyoneDegreesCollectionFile *file,
	const CollectionKey *key,
	fiftyoneDegreesData *data,
	fiftyoneDegreesException *exception) {
	Component component = { 0, 0, 0, 0, { 0, 0 } };
	return CollectionReadFileVariable(
		file,
		data,
		key,
		&component,
		exception);
}

#endif

void fiftyoneDegreesComponentInitList(
	fiftyoneDegreesCollection *components,
	fiftyoneDegreesList *list,
	uint32_t count,
	fiftyoneDegreesException *exception) {
	uint32_t offset = 0;
	Item item;
	Component *component;
	if (ListInit(list, count) == list) {
		CollectionKeyType keyType = {
			FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_COMPONENT,
			0, // TBD
			fiftyoneDegreesComponentGetFinalSize,
		};
		while (list->count < count && EXCEPTION_OKAY) {

			// Get the component and add it to the list.
			DataReset(&item.data);
			keyType.initialBytesCount = sizeof(Component) - sizeof(fiftyoneDegreesComponentKeyValuePair);
			const CollectionKey componentKey = {
				offset,
				&keyType,
			};
			component = (Component*)components->get(
				components,
				&componentKey,
				&item,
				exception);
			if (component != NULL && EXCEPTION_OKAY) {
				ListAdd(list, &item);

				// Move to the next component in the collection.
				offset += fiftyoneDegreesComponentGetFinalSize(
					(void*)component,
					exception);
			}
		}
	}
}

fiftyoneDegreesHeaderPtrs* fiftyoneDegreesComponentGetHeaders(
	fiftyoneDegreesComponent* component,
	fiftyoneDegreesHeaders* headers,
	fiftyoneDegreesException* exception) {
	const ComponentKeyValuePair* keyValue;
	HeaderPtrs* componentHeaders;
	
	// Create an array of header pointers.
	FIFTYONE_DEGREES_ARRAY_CREATE(
		fiftyoneDegreesHeaderPtr,
		componentHeaders,
		component->keyValuesCount);
	if (componentHeaders == NULL) {
		EXCEPTION_SET(INSUFFICIENT_MEMORY);
		return NULL;
	}

	// Add the header from the headers array that relate to each header if the
	// component considers.
	for (uint32_t i = 0; i < component->keyValuesCount; i++) {
		keyValue = ComponentGetKeyValuePair(
			component, 
			(uint16_t)i, 
			exception);
		componentHeaders->items[i] = 
			HeadersGetHeaderFromUniqueId(headers, keyValue->key);
		componentHeaders->count++;
	}
	assert(componentHeaders->count == componentHeaders->capacity);

	return componentHeaders;
}