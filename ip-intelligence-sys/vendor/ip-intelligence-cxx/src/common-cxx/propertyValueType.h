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

#ifndef FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_H_INCLUDED
#define FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_H_INCLUDED

#include "common.h"

/**
 * Enum of property types.
 */
typedef enum e_fiftyone_degrees_property_value_type {
	FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING = 0, /**< String */
	FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_INTEGER = 1, /**< Integer */
	FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_DOUBLE = 2, /**< Double */
	FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_BOOLEAN = 3, /**< Boolean */
	FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_JAVASCRIPT = 4, /**< JavaScript string */
	FIFTYONE_DEGREES_PROPERTY_VALUE_SINGLE_PRECISION_FLOAT = 5, /**< Single precision floating point value */
	FIFTYONE_DEGREES_PROPERTY_VALUE_SINGLE_BYTE = 6, /**< Single byte value */
	FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_COORDINATE = 7, /**< Coordinate */
	FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_IP_ADDRESS = 8, /**< Ip Range */
	FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_WKB = 9, /**< Well-known binary for geometry */
	FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_OBJECT = 10, /**<  Mainly this is used for nested AspectData. */
	/**
	 * Angle north (positive) or south (negative) of the [celestial] equator,
	 * [-90;90] saved as short (int16_t),
	 * i.e. projected onto [-INT16_MAX;INT16_MAX]
	 */
	FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_DECLINATION = 11,
	/**
	 * Horizontal angle from a cardinal direction (e.g. 0 meridian),
	 * [-180;180] saved as short (int16_t),
	 * i.e. projected onto [-INT16_MAX;INT16_MAX]
	 */
	FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_AZIMUTH = 12,
	FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_WKB_R = 13, /**< Well-known binary (reduced) for geometry */
	FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_WEIGHTED_STRING = 14, /**< Weighted list of String values. */
	FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_WEIGHTED_INT = 15, /**< Weighted list of Int values. */
	FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_WEIGHTED_DOUBLE = 16, /**< Weighted list of Double values. */
	FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_WEIGHTED_BOOL = 17, /**< Weighted list of Bool values. */
	FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_WEIGHTED_SINGLE = 18, /**< Weighted list of Single values. */
	FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_WEIGHTED_BYTE = 19, /**< Weighted list of Byte values. */
	FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_WEIGHTED_IP_ADDRESS= 20, /**< Weighted list of Ip range values. */
	/**
	 * Weighted list of Well-known binary for geometry (reduced) values.
	 */
	FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_WEIGHTED_WKB_R = 21,
	// Non-Property Collection Value Types
	FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_CUSTOM = 1000, /**< Reservation start. Should not be used. */
	FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_VALUE, /**< fiftyoneDegreesValue */
	FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_PROFILE, /**< fiftyoneDegreesProfile */
	FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_PROFILE_OFFSET, /**< fiftyoneDegreesProfileOffset */
	FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_COMPONENT, /**< fiftyoneDegreesComponent */
	FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_PROPERTY, /**< fiftyoneDegreesProperty */
	FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_PROPERTY_TYPE_RECORD, /**< fiftyoneDegreesPropertyTypeRecord */
	FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_IPV4_RANGE, /**< fiftyoneDegreesIpv4Range */
	FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_IPV6_RANGE, /**< fiftyoneDegreesIpv6Range */
	FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_OFFSET_PERCENTAGE, /**< offsetPercentage (in ipi.c) */
	FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_GRAPH_DATA_CLUSTER, /**< Cluster (in graph.c) */
	FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_GRAPH_DATA_SPAN, /**< Span (in graph.c) */
	FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_GRAPH_DATA_SPAN_BYTES, /**< bytes of Span (in graph.c) */
	FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_GRAPH_DATA_NODE_BYTES, /**< bytes of node (in graph.c) */
	FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_GRAPH_INFO, /**< fiftyoneDegreesIpiCgInfo */
} fiftyoneDegreesPropertyValueType;

/**
 * Returns the underlying (non-weighted) stored value type for a given
 * property value type. For weighted types (e.g., WeightedString), returns
 * the base type (e.g., String). For non-weighted types, returns the input
 * unchanged.
 * @param type the property value type to get the underlying type for
 * @return the underlying type, or the input type if not weighted
 */
EXTERNAL fiftyoneDegreesPropertyValueType
fiftyoneDegreesPropertyValueTypeGetUnderlyingType(
    fiftyoneDegreesPropertyValueType type);

#endif
