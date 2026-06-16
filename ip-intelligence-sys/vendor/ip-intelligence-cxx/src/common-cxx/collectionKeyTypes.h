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

#ifndef FIFTYONE_DEGREES_COLLECTION_KEY_TYPES_H_INCLUDED
#define FIFTYONE_DEGREES_COLLECTION_KEY_TYPES_H_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesCollectionKeyTypes CollectionKeyTypes
 *
 * Group of related items such as collection key type constants.
 *
 * @{
 */

#include "collectionKey.h"
#include "common.h"
#include "component.h"
#include "exceptions.h"
#include "profile.h"

#ifndef FIFTYONE_DEGREES_MEMORY_ONLY
static uint32_t fiftyoneDegreesGetFinalByteArraySize(
    const void *initial,
    fiftyoneDegreesException * const exception) {
#	ifdef _MSC_VER
    UNREFERENCED_PARAMETER(exception);
#	endif
    return (uint32_t)(sizeof(int16_t) + (*(int16_t*)initial));
}
#else
#define fiftyoneDegreesGetFinalByteArraySize NULL
#endif

#ifndef FIFTYONE_DEGREES_MEMORY_ONLY
EXTERNAL uint32_t fiftyoneDegreesThrowUnsupportedStoredValueType(
    const void *initial,
    fiftyoneDegreesException *exception);
#else
#define fiftyoneDegreesThrowUnsupportedStoredValueType NULL
#endif


static const fiftyoneDegreesCollectionKeyType CollectionKeyType_Azimuth_raw = {
    FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_AZIMUTH,
    sizeof(int16_t),
    NULL,
};
static const fiftyoneDegreesCollectionKeyType * const CollectionKeyType_Azimuth = &CollectionKeyType_Azimuth_raw;
static const fiftyoneDegreesCollectionKeyType CollectionKeyType_Byte_raw = {
    FIFTYONE_DEGREES_PROPERTY_VALUE_SINGLE_BYTE,
    sizeof(uint8_t),
    NULL,
};
static const fiftyoneDegreesCollectionKeyType * const CollectionKeyType_Byte = &CollectionKeyType_Byte_raw;
static const fiftyoneDegreesCollectionKeyType CollectionKeyType_Component_raw = {
    FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_COMPONENT,
    sizeof(fiftyoneDegreesComponent) - sizeof(fiftyoneDegreesComponentKeyValuePair),
    fiftyoneDegreesComponentGetFinalSize,
};
static const fiftyoneDegreesCollectionKeyType * const CollectionKeyType_Component = &CollectionKeyType_Component_raw;
static const fiftyoneDegreesCollectionKeyType CollectionKeyType_Declination_raw = {
    FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_DECLINATION,
    sizeof(int16_t),
    NULL,
};
static const fiftyoneDegreesCollectionKeyType * const CollectionKeyType_Declination = &CollectionKeyType_Declination_raw;
static const fiftyoneDegreesCollectionKeyType CollectionKeyType_Integer_raw = {
    FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_INTEGER,
    sizeof(uint32_t),
    NULL,
};
static const fiftyoneDegreesCollectionKeyType * const CollectionKeyType_Integer = &CollectionKeyType_Integer_raw;
static const fiftyoneDegreesCollectionKeyType CollectionKeyType_IPAddress_raw = {
    FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_IP_ADDRESS,
    sizeof(uint16_t),
    fiftyoneDegreesGetFinalByteArraySize,
};
static const fiftyoneDegreesCollectionKeyType * const CollectionKeyType_IPAddress = &CollectionKeyType_IPAddress_raw;
static const fiftyoneDegreesCollectionKeyType CollectionKeyType_Float_raw = {
    FIFTYONE_DEGREES_PROPERTY_VALUE_SINGLE_PRECISION_FLOAT,
    sizeof(fiftyoneDegreesFloat),
    NULL,
};
static const fiftyoneDegreesCollectionKeyType * const CollectionKeyType_Float = &CollectionKeyType_Float_raw;
static const fiftyoneDegreesCollectionKeyType CollectionKeyType_Profile_raw = {
    FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_PROFILE,
    sizeof(fiftyoneDegreesProfile),
    fiftyoneDegreesProfileGetFinalSize,
};
static const fiftyoneDegreesCollectionKeyType * const CollectionKeyType_Profile = &CollectionKeyType_Profile_raw;
static const fiftyoneDegreesCollectionKeyType CollectionKeyType_ProfileOffset_raw = {
    FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_PROFILE_OFFSET,
    sizeof(fiftyoneDegreesProfileOffset),
    NULL,
};
static const fiftyoneDegreesCollectionKeyType * const CollectionKeyType_ProfileOffset = &CollectionKeyType_ProfileOffset_raw;
static const fiftyoneDegreesCollectionKeyType CollectionKeyType_Property_raw = {
    FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_PROPERTY,
    sizeof(fiftyoneDegreesProperty),
    NULL,
};
static const fiftyoneDegreesCollectionKeyType * const CollectionKeyType_Property = &CollectionKeyType_Property_raw;
static const fiftyoneDegreesCollectionKeyType CollectionKeyType_PropertyTypeRecord_raw = {
    FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_PROPERTY_TYPE_RECORD,
    sizeof(fiftyoneDegreesPropertyTypeRecord),
    NULL,
};
static const fiftyoneDegreesCollectionKeyType * const CollectionKeyType_PropertyTypeRecord = &CollectionKeyType_PropertyTypeRecord_raw;
static const fiftyoneDegreesCollectionKeyType CollectionKeyType_String_raw = {
    FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING,
    sizeof(uint16_t),
    fiftyoneDegreesStringGetFinalSize,
};
static const fiftyoneDegreesCollectionKeyType * const CollectionKeyType_String = &CollectionKeyType_String_raw;
static const fiftyoneDegreesCollectionKeyType CollectionKeyType_Unsupported_raw = {
    FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_CUSTOM,
    1,
    fiftyoneDegreesThrowUnsupportedStoredValueType,
};
static const fiftyoneDegreesCollectionKeyType * const CollectionKeyType_Unsupported = &CollectionKeyType_Unsupported_raw;
static const fiftyoneDegreesCollectionKeyType CollectionKeyType_Value_raw = {
    FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_VALUE,
    sizeof(fiftyoneDegreesValue),
    NULL,
};
static const fiftyoneDegreesCollectionKeyType * const CollectionKeyType_Value = &CollectionKeyType_Value_raw;
static const fiftyoneDegreesCollectionKeyType CollectionKeyType_WKB_raw = {
    FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_WKB,
    sizeof(uint16_t),
    fiftyoneDegreesGetFinalByteArraySize,
};
static const fiftyoneDegreesCollectionKeyType * const CollectionKeyType_WKB = &CollectionKeyType_WKB_raw;
static const fiftyoneDegreesCollectionKeyType CollectionKeyType_WKB_R_raw = {
    FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_WKB_R,
    sizeof(uint16_t),
    fiftyoneDegreesGetFinalByteArraySize,
};
static const fiftyoneDegreesCollectionKeyType * const CollectionKeyType_WKB_R = &CollectionKeyType_WKB_R_raw;

EXTERNAL const fiftyoneDegreesCollectionKeyType *fiftyoneDegreesGetCollectionKeyTypeForStoredValueType(
    fiftyoneDegreesPropertyValueType storedValueType,
    fiftyoneDegreesException *exception);

#define GetCollectionKeyTypeForStoredValueType fiftyoneDegreesGetCollectionKeyTypeForStoredValueType /**< Synonym for #fiftyoneDegreesGetCollectionKeyTypeForStoredValueType function. */

/**
 * @}
 */

#endif
