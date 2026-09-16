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

#ifndef FIFTYONE_DEGREES_SYNONYM_IPI_INCLUDED
#define FIFTYONE_DEGREES_SYNONYM_IPI_INCLUDED

#include "ipi.h"
#include "constantsIpi.h"
#include "ipi_weighted_results.h"
#include "common-cxx/fiftyone.h"

// Data types
MAP_TYPE(WeightedItem)
MAP_TYPE(WeightedItemList)
MAP_TYPE(ResultIpi)
MAP_TYPE(ResultsIpi)
MAP_TYPE(ResultIpiArray)
MAP_TYPE(ConfigIpi)
MAP_TYPE(DataSetIpi)
MAP_TYPE(DataSetIpiHeader)
MAP_TYPE(Ipv4Range)
MAP_TYPE(Ipv6Range)
MAP_TYPE(CombinationProfileIndex)
MAP_TYPE(ResultProfileIndex)
MAP_TYPE(WeightedValueHeader)
MAP_TYPE(WeightedInt)
MAP_TYPE(WeightedDouble)
MAP_TYPE(WeightedBool)
MAP_TYPE(WeightedByte)
MAP_TYPE(WeightedString)
MAP_TYPE(WeightedValuesCollection)

// Methods
#define ResultsIpiCreate fiftyoneDegreesResultsIpiCreate /**< Synonym for #fiftyoneDegreesResultsIpiCreate function. */
#define ResultsIpiFree fiftyoneDegreesResultsIpiFree /**< Synonym for #fiftyoneDegreesResultsIpiFree function. */
#define ResultsIpiFromIpAddress fiftyoneDegreesResultsIpiFromIpAddress /**< Synonym for #fiftyoneDegreesResultsIpiFromIpAddress function. */
#define ResultsIpiFromIpAddressString fiftyoneDegreesResultsIpiFromIpAddressString /**< Synonym for #fiftyoneDegreesResultsIpiFromIpAddressString function. */
#define ResultsIpiFromEvidence fiftyoneDegreesResultsIpiFromEvidence /**< Synonym for #fiftyoneDegreesResultsIpiFromEvidence function. */
#define ResultsIpiGetValues fiftyoneDegreesResultsIpiGetValues /**< Synonym for #fiftyoneDegreesResultsIpiGetValues function. */
#define ResultsIpiAddValuesString fiftyoneDegreesResultsIpiAddValuesString /**< Synonym for #fiftyoneDegreesResultsIpiAddValuesString function. */
#define ResultsIpiGetValuesString fiftyoneDegreesResultsIpiGetValuesString /**< Synonym for #fiftyoneDegreesResultsIpiGetValuesString function. */
#define ResultsIpiGetValuesStringByRequiredPropertyIndex fiftyoneDegreesResultsIpiGetValuesStringByRequiredPropertyIndex /**< Synonym for #fiftyoneDegreesResultsIpiGetValuesStringByRequiredPropertyIndex function. */
#define ResultsIpiGetHasValues fiftyoneDegreesResultsIpiGetHasValues /**< Synonym for #fiftyoneDegreesResultsIpiGetHasValues function. */
#define ResultsIpiGetNoValueReason fiftyoneDegreesResultsIpiGetNoValueReason /**< Synonym for #fiftyoneDegreesResultsIpiGetNoValueReason function. */
#define ResultsIpiGetNoValueReasonMessage fiftyoneDegreesResultsIpiGetNoValueReasonMessage /**< Synonym for #fiftyoneDegreesResultsIpiGetNoValueReasonMessage function. */
#define IpiInitManagerFromFile fiftyoneDegreesIpiInitManagerFromFile /**< Synonym for #fiftyoneDegreesIpiInitManagerFromFile function. */
#define IpiInitManagerFromMemory fiftyoneDegreesIpiInitManagerFromMemory /**< Synonym for #fiftyoneDegreesIpiInitManagerFromMemory function. */
#define DataSetIpiGet fiftyoneDegreesDataSetIpiGet /**< Synonym for #fiftyoneDegreesDataSetIpiGet function. */
#define DataSetIpiRelease fiftyoneDegreesDataSetIpiRelease /**< Synonym for #fiftyoneDegreesDataSetIpiRelease function. */
#define IpiReloadManagerFromOriginalFile fiftyoneDegreesIpiReloadManagerFromOriginalFile /**< Synonym for #fiftyoneDegreesIpiReloadManagerFromOriginalFile function. */
#define IpiReloadManagerFromFile fiftyoneDegreesIpiReloadManagerFromFile /**< Synonym for #fiftyoneDegreesIpiReloadManagerFromFile function. */
#define IpiReloadManagerFromMemory fiftyoneDegreesIpiReloadManagerFromMemory /**< Synonym for #fiftyoneDegreesIpiReloadManagerFromMemory function. */
#define IpiGetNetworkIdFromResult fiftyoneDegreesIpiGetNetworkIdFromResult /**< Synonym for #fiftyoneDegreesIpiGetNetworkIdFromResult function. */
#define IpiGetNetworkIdFromResults fiftyoneDegreesIpiGetNetworkIdFromResults /**< Synonym for #fiftyoneDegreesIpiGetNetworkIdFromResults function. */
#define IpiGetIpAddressAsString fiftyoneDegreesIpiGetIpAddressAsString /**< Synonym for #fiftyoneDegreesIpiGetIpAddressAsString function. */
#define IpiGetIpAddressAsByteArray fiftyoneDegreesIpiGetIpAddressAsByteArray /**< Synonym for #fiftyoneDegreesIpiGetIpAddressAsByteArray function. */
#define IpiIterateProfilesForPropertyAndValue fiftyoneDegreesIpiIterateProfilesForPropertyAndValue /**< Synonym for #fiftyoneDegreesIpiIterateProfilesForPropertyAndValue function. */
#define ResultsIpiGetValuesCollection fiftyoneDegreesResultsIpiGetValuesCollection /**< Synonym for #fiftyoneDegreesResultsIpiGetValuesCollection function. */
#define WeightedValuesCollectionRelease fiftyoneDegreesWeightedValuesCollectionRelease /**< Synonym for #fiftyoneDegreesWeightedValuesCollectionRelease function. */

// Constants
#define DefaultWktDecimalPlaces fiftyoneDegreesDefaultWktDecimalPlaces /**< Synonym for #fiftyoneDegreesDefaultWktDecimalPlaces config. */

// Config
#define IpiInMemoryConfig fiftyoneDegreesIpiInMemoryConfig /**< Synonym for #fiftyoneDegreesIpiInMemoryConfig config. */
#define IpiHighPerformanceConfig fiftyoneDegreesIpiHighPerformanceConfig /**< Synonym for #fiftyoneDegreesIpiHighPerformanceConfig config. */
#define IpiLowMemoryConfig fiftyoneDegreesIpiLowMemoryConfig /**< Synonym for #fiftyoneDegreesIpiLowMemoryConfig config. */
#define IpiBalancedConfig fiftyoneDegreesIpiBalancedConfig /**< Synonym for #fiftyoneDegreesIpiBalancedConfig config. */
#define IpiBalancedTempConfig fiftyoneDegreesIpiBalancedTempConfig /**< Synonym for #fiftyoneDegreesIpiBalancedTempConfig config. */
#define IpiDefaultConfig fiftyoneDegreesIpiDefaultConfig /**< Synonym for #fiftyoneDegreesIpiDefaultConfig config. */

#endif
