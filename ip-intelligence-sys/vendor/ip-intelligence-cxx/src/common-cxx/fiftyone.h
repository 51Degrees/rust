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

/**
 * @defgroup FiftyOneDegreesCommon Common
 *
 * Common 51Degrees methods, types and macros.
 */

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesSynonyms Synonyms
 *
 * Quick shortenings of common methods and types.
 *
 * ## Introduction
 *
 * To ensure compatibility with larger projects where naming conflicts could
 * occur this aliases file enables references to 51degrees functions, 
 * structures and types to be made without requiring the prefix
 * `fiftyoneDegrees` to be provided. This is similar to the use of namespaces
 * in languages which support them and leads to code that is shorter and easier
 * to read.
 *
 * @{
 */

#ifndef FIFTYONE_DEGREES_SYNONYM_COMMON_INCLUDED
#define FIFTYONE_DEGREES_SYNONYM_COMMON_INCLUDED

#include "exceptions.h"
#include "cache.h"
#include "collection.h"
#include "component.h"
#include "config.h"
#include "data.h"
#include "dataset.h"
#include "date.h"
#include "evidence.h"
#include "file.h"
#include "headers.h"
#include "list.h"
#include "memory.h"
#include "pool.h"
#include "properties.h"
#include "resource.h"
#include "results.h"
#include "status.h"
#include "storedBinaryValue.h"
#include "string.h"
#include "property.h"
#include "value.h"
#include "profile.h"
#include "overrides.h"
#include "tree.h"
#include "ip.h"
#include "float.h"
#include "snprintf.h"
#include "bool.h"
#include "process.h"
#include "pair.h"
#include "yamlfile.h"
#include "indices.h"
#include "json.h"
#include "wkbtot.h"
#include "constants.h"
#include "weightedItem.h"
#include "propertyValueType.h"

/**
 * Macro used to support synonym implementation. Creates a typedef which 
 * removes the prefix `fiftyoneDegrees`.
 */
#define MAP_TYPE(t) typedef fiftyoneDegrees##t t; /**< Synonym for #fiftyoneDegrees##t type. */

/* <-- only one asterisk to avoid inclusion in documentation
 * Most mappings simply remove fiftyoneDegrees. However in some cases even 
 * shorter names make sense. These are provided before the general references
 * to MAP_TYPE.
 */

typedef fiftyoneDegreesCollectionItem Item; /**< Synonym for #fiftyoneDegreesCollectionItem type. */

/* <-- only one asterisk to avoid inclusion in documentation
 * General references to MAP_TYPE used to strip fiftyoneDegrees as a prefix
 * from types.
 */

MAP_TYPE(Exception);
MAP_TYPE(CollectionIndexOrOffset)
MAP_TYPE(CollectionKey)
MAP_TYPE(CollectionKeyType)
MAP_TYPE(CollectionIterateMethod)
MAP_TYPE(CollectionMemory)
#ifndef FIFTYONE_DEGREES_MEMORY_ONLY
MAP_TYPE(CollectionFile)
MAP_TYPE(CollectionFileRead)
MAP_TYPE(CollectionCache)
MAP_TYPE(CollectionFileRead)
#endif
MAP_TYPE(FileHandle)
MAP_TYPE(FileOffset)
MAP_TYPE(FileOffsetUnsigned)
MAP_TYPE(CacheNode)
MAP_TYPE(FilePool)
MAP_TYPE(CollectionHeader)
MAP_TYPE(Data)
MAP_TYPE(Cache)
MAP_TYPE(MemoryReader)
MAP_TYPE(CacheShard)
MAP_TYPE(StatusCode)
MAP_TYPE(PropertiesRequired)
MAP_TYPE(DataSetBase)
MAP_TYPE(ResourceManager)
MAP_TYPE(EvidenceKeyValuePair)
MAP_TYPE(EvidencePrefixMap)
MAP_TYPE(EvidencePrefix)
MAP_TYPE(Headers)
MAP_TYPE(ResourceHandle)
MAP_TYPE(InterlockDoubleWidth)
MAP_TYPE(Pool)
MAP_TYPE(PoolResourceCreate)
MAP_TYPE(PoolResourceFree)
MAP_TYPE(PoolItem)
MAP_TYPE(PoolHead)
MAP_TYPE(PoolResourceSize)
MAP_TYPE(List)
MAP_TYPE(DataSetInitFromFileMethod)
MAP_TYPE(DataSetInitFromMemoryMethod)
MAP_TYPE(DataSetInitFromMemoryMethod)
MAP_TYPE(PropertiesGetMethod)
MAP_TYPE(HeadersGetMethod)
MAP_TYPE(DataSetInitFromFileMethod)
MAP_TYPE(String)
MAP_TYPE(VarLengthByteArray)
MAP_TYPE(StoredBinaryValue)
MAP_TYPE(Property)
MAP_TYPE(PropertyTypeRecord)
MAP_TYPE(Component)
MAP_TYPE(ComponentKeyValuePair)
MAP_TYPE(Value)
MAP_TYPE(Profile)
MAP_TYPE(OverrideProperty)
MAP_TYPE(OverridePropertyArray)
MAP_TYPE(OverrideValue)
MAP_TYPE(OverrideValueArray)
MAP_TYPE(EvidenceKeyValuePair)
MAP_TYPE(EvidenceKeyValuePairArray)
MAP_TYPE(PropertyAvailable)
MAP_TYPE(PropertyValueType)
MAP_TYPE(PropertiesAvailable)
MAP_TYPE(EvidencePropertyIndex)
MAP_TYPE(EvidenceProperties)
MAP_TYPE(Header)
MAP_TYPE(Headers)
MAP_TYPE(HeaderID)
MAP_TYPE(HeaderPtr)
MAP_TYPE(HeaderPtrs)
MAP_TYPE(OverridesFilterMethod)
MAP_TYPE(Mutex)
MAP_TYPE(Signal)
MAP_TYPE(TreeNode)
MAP_TYPE(TreeRoot)
MAP_TYPE(ProfileOffset)
MAP_TYPE(ProfileIterateMethod)
MAP_TYPE(ProfileValuesCountType)
MAP_TYPE(Float)
MAP_TYPE(KeyValuePair)
MAP_TYPE(HeaderID)
MAP_TYPE(IndicesPropertyProfile)
MAP_TYPE(StringBuilder)
MAP_TYPE(Json)
MAP_TYPE(KeyValuePairArray)
MAP_TYPE(IpType)
MAP_TYPE(IpAddress)
MAP_TYPE(WkbtotResult)
MAP_TYPE(WkbtotReductionMode)
MAP_TYPE(WeightedItem)
MAP_TYPE(WeightedItemList)

#define ProfileGetFinalSize fiftyoneDegreesProfileGetFinalSize /**< Synonym for #fiftyoneDegreesProfileGetFinalSize function. */
#define ProfileGetOffsetForProfileId fiftyoneDegreesProfileGetOffsetForProfileId /**< Synonym for #fiftyoneDegreesProfileGetOffsetForProfileId function. */
#define OverrideValuesAdd fiftyoneDegreesOverrideValuesAdd /**< Synonym for #fiftyoneDegreesOverrideValuesAdd function. */
#define ExceptionGetMessage fiftyoneDegreesExceptionGetMessage /**< Synonym for #fiftyoneDegreesExceptionGetMessage function. */
#define ProfileGetByProfileId fiftyoneDegreesProfileGetByProfileId /**< Synonym for #fiftyoneDegreesProfileGetByProfileId function. */
#define ProfileGetByProfileIdIndirect fiftyoneDegreesProfileGetByProfileIdIndirect /**< Synonym for #fiftyoneDegreesProfileGetByProfileIdIndirect function. */
#define ProfileGetByIndex fiftyoneDegreesProfileGetByIndex /**< Synonym for #fiftyoneDegreesProfileGetByIndex function. */
#define OverridesAdd fiftyoneDegreesOverridesAdd /**< Synonym for #fiftyoneDegreesOverridesAdd function. */
#define OverrideProfileIds fiftyoneDegreesOverrideProfileIds /**< Synonym for #fiftyoneDegreesOverrideProfileIds function. */
#define OverridePropertiesFree fiftyoneDegreesOverridePropertiesFree /**< Synonym for #fiftyoneDegreesOverridePropertiesFree function. */
#define ComponentInitList fiftyoneDegreesComponentInitList /**< Synonym for #fiftyoneDegreesComponentInitList function. */
#define ComponentGetFinalSize fiftyoneDegreesComponentGetFinalSize /**< Synonym for #fiftyoneDegreesComponentGetFinalSize function. */
#define ComponentGetHeaders fiftyoneDegreesComponentGetHeaders /**< Synonym for #fiftyoneDegreesComponentGetHeaders function. */
#define CollectionGetInteger32 fiftyoneDegreesCollectionGetInteger32 /**< Synonym for #fiftyoneDegreesCollectionGetInteger32 function. */
#define PropertyGet fiftyoneDegreesPropertyGet /**< Synonym for #fiftyoneDegreesPropertyGet function. */
#define ProfileIterateValuesForProperty fiftyoneDegreesProfileIterateValuesForProperty /**< Synonym for #fiftyoneDegreesProfileIterateValuesForProperty function. */
#define ProfileIterateValuesForPropertyWithIndex fiftyoneDegreesProfileIterateValuesForPropertyWithIndex /**< Synonym for #fiftyoneDegreesProfileIterateValuesForPropertyWithIndex function. */
#define ProfileIterateValueIndexes fiftyoneDegreesProfileIterateValueIndexes /**< Synonym for #fiftyoneDegreesProfileIterateValueIndexes function. */
#define ProfileIterateProfilesForPropertyAndValue fiftyoneDegreesProfileIterateProfilesForPropertyAndValue /**< Synonym for #fiftyoneDegreesProfileIterateProfilesForPropertyAndValue function. */
#define ProfileIterateProfilesForPropertyWithTypeAndValue fiftyoneDegreesProfileIterateProfilesForPropertyWithTypeAndValue /**< Synonym for #fiftyoneDegreesProfileIterateProfilesForPropertyWithTypeAndValue function. */
#define ProfileIterateProfilesForPropertyWithTypeAndValueAndOffsetExtractor fiftyoneDegreesProfileIterateProfilesForPropertyWithTypeAndValueAndOffsetExtractor /**< Synonym for #fiftyoneDegreesProfileIterateProfilesForPropertyWithTypeAndValueAndOffsetExtractor function. */
#define ProfileOffsetAsPureOffset fiftyoneDegreesProfileOffsetAsPureOffset /**< Synonym for #fiftyoneDegreesProfileOffsetAsPureOffset function. */
#define ProfileOffsetToPureOffset fiftyoneDegreesProfileOffsetToPureOffset /**< Synonym for #fiftyoneDegreesProfileOffsetToPureOffset function. */
#define PropertiesGetPropertyIndexFromName fiftyoneDegreesPropertiesGetPropertyIndexFromName /**< Synonym for #fiftyoneDegreesPropertiesGetPropertyIndexFromName function. */
#define TreeIterate fiftyoneDegreesTreeIterateNodes /**< Synonym for #fiftyoneDegreesTreeIterateNodes function. */
#define TreeCount fiftyoneDegreesTreeCount /**< Synonym for #fiftyoneDegreesTreeCount function. */
#define TreeDelete fiftyoneDegreesTreeDelete /**< Synonym for #fiftyoneDegreesTreeDelete function. */
#define TreeInsert fiftyoneDegreesTreeInsert /**< Synonym for #fiftyoneDegreesTreeInsert function. */
#define TreeFind fiftyoneDegreesTreeFind /**< Synonym for #fiftyoneDegreesTreeFind function. */
#define TreeNodeInit fiftyoneDegreesTreeNodeInit /**< Synonym for #fiftyoneDegreesTreeNodeInit function. */
#define TreeNodeRemove fiftyoneDegreesTreeNodeRemove /**< Synonym for #fiftyoneDegreesTreeNodeRemove function. */
#define TreeRootInit fiftyoneDegreesTreeRootInit /**< Synonym for #fiftyoneDegreesTreeRootInit function. */
#define OverridesGetOverridingRequiredPropertyIndex fiftyoneDegreesOverridesGetOverridingRequiredPropertyIndex /**< Synonym for #fiftyoneDegreesOverridesGetOverridingRequiredPropertyIndex function. */
#define OverridePropertiesCreate fiftyoneDegreesOverridePropertiesCreate /**< Synonym for #fiftyoneDegreesOverridePropertiesCreate function. */
#define EvidenceCreate fiftyoneDegreesEvidenceCreate /**< Synonym for #fiftyoneDegreesEvidenceCreate function. */
#define EvidenceFree fiftyoneDegreesEvidenceFree /**< Synonym for #fiftyoneDegreesEvidenceFree function. */
#define OverridesGetOverridingRequiredPropertyIndex fiftyoneDegreesOverridesGetOverridingRequiredPropertyIndex /**< Synonym for #fiftyoneDegreesOverridesGetOverridingRequiredPropertyIndex function. */
#define StringCompareLength fiftyoneDegreesStringCompareLength /**< Synonym for #fiftyoneDegreesStringCompareLength function. */
#define StringCompare fiftyoneDegreesStringCompare /**< Synonym for #fiftyoneDegreesStringCompare function. */
#define StringSubString fiftyoneDegreesStringSubString /**< Synonym for #fiftyoneDegreesSubString function. */
#define OverridesExtractFromEvidence fiftyoneDegreesOverridesExtractFromEvidence /**< Synonym for #fiftyoneDegreesOverridesExtractFromEvidence function. */
#define EvidenceIterate fiftyoneDegreesEvidenceIterate /**< Synonym for #fiftyoneDegreesEvidenceIterate function. */
#define EvidenceIterateForHeaders fiftyoneDegreesEvidenceIterateForHeaders /**< Synonym for #fiftyoneDegreesEvidenceIterateForHeaders function. */
#define CacheRelease fiftyoneDegreesCacheRelease /**< Synonym for #fiftyoneDegreesCacheRelease function. */
#define DataReset fiftyoneDegreesDataReset /**< Synonym for #fiftyoneDegreesDataReset function. */
#define CacheFree fiftyoneDegreesCacheFree /**< Synonym for #fiftyoneDegreesCacheFree function. */
#define FileHandleGet fiftyoneDegreesFileHandleGet /**< Synonym for #fiftyoneDegreesFileHandleGet function. */
#define FileHandleRelease fiftyoneDegreesFileHandleRelease /**< Synonym for #fiftyoneDegreesFileHandleRelease function. */
#define DataMalloc fiftyoneDegreesDataMalloc /**< Synonym for #fiftyoneDegreesDataMalloc function. */
#define CacheGet fiftyoneDegreesCacheGet /**< Synonym for #fiftyoneDegreesCacheGet function. */
#define CacheCreate fiftyoneDegreesCacheCreate /**< Synonym for #fiftyoneDegreesCacheCreate function. */
#define MemoryAdvance fiftyoneDegreesMemoryAdvance /**< Synonym for #fiftyoneDegreesMemoryAdvance function. */
#define MemoryTrackingReset fiftyoneDegreesMemoryTrackingReset /**< Synonym for #fiftyoneDegreesMemoryTrackingReset function. */
#define MemoryTrackingGetMax fiftyoneDegreesMemoryTrackingGetMax /**< Synonym for #fiftyoneDegreesMemoryTrackingGetMax function. */
#define MemoryTrackingGetAllocated fiftyoneDegreesMemoryTrackingGetAllocated /**< Synonym for #fiftyoneDegreesMemoryTrackingGetAllocated function. */
#define SetUpMemoryTracking fiftyoneDegreesSetUpMemoryTracking /**< Synonym for #fiftyoneDegreesSetUpMemoryTracking function. */
#define UnsetMemoryTracking fiftyoneDegreesUnsetMemoryTracking /**< Synonym for #fiftyoneDegreesUnsetMemoryTracking function. */
#define Malloc fiftyoneDegreesMalloc /**< Synonym for #fiftyoneDegreesMalloc function. */
#define MallocAligned fiftyoneDegreesMallocAligned /**< Synonym for #fiftyoneDegreesMallocAligned function. */
#define Free fiftyoneDegreesFree /**< Synonym for #fiftyoneDegreesFree function. */
#define FreeAligned fiftyoneDegreesFreeAligned /**< Synonym for #fiftyoneDegreesFreeAligned function. */
#define ResourceHandleIncUse fiftyoneDegreesResourceHandleIncUse /**< Synonym for #fiftyoneDegreesResourceHandleIncUse function. */
#define PropertiesDefault fiftyoneDegreesPropertiesDefault /**< Synonym for #fiftyoneDegreesPropertiesDefault function. */
#define ListGetAsString fiftyoneDegreesListGetAsString /**< Synonym for #fiftyoneDegreesListGetAsString function. */
#define ListAdd fiftyoneDegreesListAdd /**< Synonym for #fiftyoneDegreesListAdd function. */
#define ListInit fiftyoneDegreesListInit /**< Synonym for #fiftyoneDegreesListInit function. */
#define ListFree fiftyoneDegreesListFree /**< Synonym for #fiftyoneDegreesListFree function. */
#define FileGetExistingTempFile fiftyoneDegreesFileGetExistingTempFile /**< Synonym for #fiftyoneDegreesFileGetExistingTempFile function. */
#define FileDeleteUnusedTempFiles fiftyoneDegreesFileDeleteUnusedTempFiles /**< Synonym for #fiftyoneDegreesFileDeleteUnusedTempFiles function. */
#define FileCreateTempFile fiftyoneDegreesFileCreateTempFile /**< Synonym for #fiftyoneDegreesFileCreateTempFile function. */
#define FileNewTempFile fiftyoneDegreesFileNewTempFile /**< Synonym for #fiftyoneDegreesFileNewTempFile function. */
#define HeadersFree fiftyoneDegreesHeadersFree /**< Synonym for #fiftyoneDegreesHeadersFree function. */
#define PropertiesFree fiftyoneDegreesPropertiesFree /**< Synonym for #fiftyoneDegreesPropertiesFree function. */
#define FilePoolRelease fiftyoneDegreesFilePoolRelease /**< Synonym for #fiftyoneDegreesFilePoolRelease function. */
#define FileSeek fiftyoneDegreesFileSeek /**< Synonym for #fiftyoneDegreesFileSeek function. */
#define FileTell fiftyoneDegreesFileTell /**< Synonym for #fiftyoneDegreesFileTell function. */
#define FileDelete fiftyoneDegreesFileDelete /**< Synonym for #fiftyoneDegreesFileDelete function. */
#define FilePoolReset fiftyoneDegreesFilePoolReset /**< Synonym for #fiftyoneDegreesFilePoolReset function. */
#define PropertiesCreate fiftyoneDegreesPropertiesCreate /**< Synonym for #fiftyoneDegreesPropertiesCreate function. */
#define HeadersIsPseudo fiftyoneDegreesHeadersIsPseudo /**< Synonym for #fiftyoneDegreesHeadersIsPseudo function. */
#define HeadersCreate fiftyoneDegreesHeadersCreate /**< Synonym for #fiftyoneDegreesHeadersCreate function. */
#define HeadersGetHeaderFromUniqueId fiftyoneDegreesHeadersGetHeaderFromUniqueId /**< Synonym for #fiftyoneDegreesHeadersGetHeaderFromUniqueId function. */
#define PseudoHeadersAddEvidence fiftyoneDegreesPseudoHeadersAddEvidence /**< Synonym for fiftyoneDegreesPseudoHeadersAddEvidence */
#define PseudoHeadersRemoveEvidence fiftyoneDegreesPseudoHeadersRemoveEvidence /**< Synonym for fiftyoneDegreesPseudoHeadersRemoveEvidence */
#define FileReadToByteArray fiftyoneDegreesFileReadToByteArray /**< Synonym for #fiftyoneDegreesFileReadToByteArray function. */
#define ResourceHandleDecUse fiftyoneDegreesResourceHandleDecUse /**< Synonym for #fiftyoneDegreesResourceHandleDecUse function. */
#define ResourceReplace fiftyoneDegreesResourceReplace /**< Synonym for #fiftyoneDegreesResourceReplace function. */
#define StatusGetMessage fiftyoneDegreesStatusGetMessage /**< Synonym for #fiftyoneDegreesStatusGetMessage function. */
#define FileOpen fiftyoneDegreesFileOpen /**< Synonym for #fiftyoneDegreesFileOpen function. */
#define PoolInit fiftyoneDegreesPoolInit /**< Synonym for #fiftyoneDegreesPoolInit function. */
#define PoolItemGet fiftyoneDegreesPoolItemGet /**< Synonym for #fiftyoneDegreesPoolItemGet function. */
#define PoolItemRelease fiftyoneDegreesPoolItemRelease /**< Synonym for #fiftyoneDegreesPoolItemRelease function. */
#define PoolFree fiftyoneDegreesPoolFree /**< Synonym for #fiftyoneDegreesPoolFree function. */
#define PoolReset fiftyoneDegreesPoolReset /**< Synonym for #fiftyoneDegreesPoolReset function. */
#define FileGetSize fiftyoneDegreesFileGetSize /**< Synonym for #fiftyoneDegreesFileGetSize function. */
#define FileCopy fiftyoneDegreesFileCopy /**< Synonym for #fiftyoneDegreesFileCopy function. */
#define MemoryTrackingMalloc fiftyoneDegreesMemoryTrackingMalloc /**< Synonym for #fiftyoneDegreesMemoryTrackingMalloc function. */
#define MemoryTrackingMallocAligned fiftyoneDegreesMemoryTrackingMallocAligned /**< Synonym for #fiftyoneDegreesMemoryTrackingMallocAligned function. */
#define MemoryTrackingFree fiftyoneDegreesMemoryTrackingFree /**< Synonym for #fiftyoneDegreesMemoryTrackingFree function. */
#define MemoryTrackingFreeAligned fiftyoneDegreesMemoryTrackingFreeAligned /**< Synonym for #fiftyoneDegreesMemoryTrackingFreeAligned function. */
#define MemoryStandardMalloc fiftyoneDegreesMemoryStandardMalloc /**< Synonym for #fiftyoneDegreesMemoryStandardMalloc function. */
#define MemoryStandardMallocAligned fiftyoneDegreesMemoryStandardMallocAligned /**< Synonym for #fiftyoneDegreesMemoryStandardMallocAligned function. */
#define MemoryStandardFree fiftyoneDegreesMemoryStandardFree /**< Synonym for #fiftyoneDegreesMemoryStandardFree function. */
#define MemoryStandardFreeAligned fiftyoneDegreesMemoryStandardFreeAligned /**< Synonym for #fiftyoneDegreesMemoryStandardFreeAligned function. */
#define ResourceManagerFree fiftyoneDegreesResourceManagerFree /**< Synonym for #fiftyoneDegreesResourceManagerFree function. */
#define StringGet fiftyoneDegreesStringGet /**< Synonym for #fiftyoneDegreesStringGet function. */
#define StringGetFinalSize fiftyoneDegreesStringGetFinalSize /**< Synonym for #fiftyoneDegreesStringGetFinalSize function. */
#define StoredBinaryValueGet fiftyoneDegreesStoredBinaryValueGet /**< Synonym for #fiftyoneDegreesStoredBinaryValueGet function. */
#define StoredBinaryValueRead fiftyoneDegreesStoredBinaryValueRead /**< Synonym for #fiftyoneDegreesStoredBinaryValueRead function. */
#define StoredBinaryValueCompareWithString fiftyoneDegreesStoredBinaryValueCompareWithString /**< Synonym for #fiftyoneDegreesStoredBinaryValueCompareWithString function. */
#define StoredBinaryValueToIntOrDefault fiftyoneDegreesStoredBinaryValueToIntOrDefault /**< Synonym for #fiftyoneDegreesStoredBinaryValueToIntOrDefault function. */
#define StoredBinaryValueToDoubleOrDefault fiftyoneDegreesStoredBinaryValueToDoubleOrDefault /**< Synonym for #fiftyoneDegreesStoredBinaryValueToDoubleOrDefault function. */
#define StoredBinaryValueToBoolOrDefault fiftyoneDegreesStoredBinaryValueToBoolOrDefault /**< Synonym for #fiftyoneDegreesStoredBinaryValueToBoolOrDefault function. */
#define EvidenceFree fiftyoneDegreesEvidenceFree /**< Synonym for #fiftyoneDegreesEvidenceFree function. */
#define EvidenceCreate fiftyoneDegreesEvidenceCreate /**< Synonym for #fiftyoneDegreesEvidenceCreate function. */
#define EvidenceMapPrefix fiftyoneDegreesEvidenceMapPrefix /**< Synonym for #fiftyoneDegreesEvidenceMapPrefix function. */
#define EvidencePrefixString fiftyoneDegreesEvidencePrefixString /**< Synonym for #fiftyoneDegreesEvidencePrefixString function. */
#define EvidenceAddPair fiftyoneDegreesEvidenceAddPair /**< Synonym for #fiftyoneDegreesEvidenceAddPair function. */
#define EvidenceAddString fiftyoneDegreesEvidenceAddString /**< Synonym for #fiftyoneDegreesEvidenceAddString function. */
#define PropertiesGetRequiredPropertyIndexFromName fiftyoneDegreesPropertiesGetRequiredPropertyIndexFromName /**< Synonym for #fiftyoneDegreesPropertiesGetRequiredPropertyIndexFromName function. */
#define PropertiesGetNameFromRequiredIndex fiftyoneDegreesPropertiesGetNameFromRequiredIndex /**< Synonym for #fiftyoneDegreesPropertiesGetNameFromRequiredIndex function. */
#define PropertiesIsSetHeaderAvailable fiftyoneDegreesPropertiesIsSetHeaderAvailable /**< Synonym for #fiftyoneDegreesPropertiesIsSetHeaderAvailable */
#define CollectionHeaderFromFile fiftyoneDegreesCollectionHeaderFromFile /**< Synonym for #fiftyoneDegreesCollectionHeaderFromFile function. */
#define CollectionCreateFromFile fiftyoneDegreesCollectionCreateFromFile /**< Synonym for #fiftyoneDegreesCollectionCreateFromFile function. */
#define CollectionHeaderFromMemory fiftyoneDegreesCollectionHeaderFromMemory /**< Synonym for #fiftyoneDegreesCollectionHeaderFromMemory function. */
#define CollectionCreateFromMemory fiftyoneDegreesCollectionCreateFromMemory /**< Synonym for #fiftyoneDegreesCollectionCreateFromMemory function. */
#define CollectionGetCount fiftyoneDegreesCollectionGetCount /**< Synonym for #fiftyoneDegreesCollectionGetCount function. */
#define FileGetPath fiftyoneDegreesFileGetPath /**< Synonym for #fiftyoneDegreesFileGetPath function. */
#define FileGetFileName fiftyoneDegreesFileGetFileName /**< Synonym for #fiftyoneDegreesFileGetFileName function. */
#define ThreadingGetIsThreadSafe fiftyoneDegreesThreadingGetIsThreadSafe /**< Synonym for #fiftyoneDegreesThreadingGetIsThreadSafe function. */
#define CollectionReadFilePosition fiftyoneDegreesCollectionReadFilePosition /**< Synonym for #fiftyoneDegreesCollectionReadFilePosition function. */
#define CollectionReadFileFixed fiftyoneDegreesCollectionReadFileFixed /**< Synonym for #fiftyoneDegreesCollectionReadFileFixed function. */
#define CollectionGetIsMemoryOnly fiftyoneDegreesCollectionGetIsMemoryOnly /**< Synonym for #fiftyoneDegreesCollectionGetIsMemoryOnly function. */
#define HeaderGetIndex fiftyoneDegreesHeaderGetIndex /**< Synonym for #fiftyoneDegreesHeaderGetIndex function. */
#define FileWrite fiftyoneDegreesFileWrite /**< Synonym for #fiftyoneDegreesFileWrite function. */
#define FilePoolInit fiftyoneDegreesFilePoolInit /**< Synonym for #fiftyoneDegreesFilePoolInit function. */
#define FileCreateDirectory fiftyoneDegreesFileCreateDirectory /**< Synonym for #fiftyoneDegreesFileCreateDirectory function. */
#define TextFileIterateWithLimit fiftyoneDegreesTextFileIterateWithLimit /**< Synonym for #fiftyoneDegreesTextFileIterateWithLimit function. */
#define TextFileIterate fiftyoneDegreesTextFileIterate /**< Synonym for #fiftyoneDegreesTextFileIterate function. */
#define ResourceManagerInit fiftyoneDegreesResourceManagerInit /**< Synonym for #fiftyoneDegreesResourceManagerInit function. */
#define PropertiesGetPropertyIndexFromRequiredIndex fiftyoneDegreesPropertiesGetPropertyIndexFromRequiredIndex /**< Synonym for #fiftyoneDegreesPropertiesGetPropertyIndexFromRequiredIndex function. */
#define DataSetRelease fiftyoneDegreesDataSetRelease /**< Synonym for #fiftyoneDegreesDataSetRelease function. */
#define DataSetReset fiftyoneDegreesDataSetReset /**< Synonym for #fiftyoneDegreesDataSetReset function. */
#define DataSetInitProperties fiftyoneDegreesDataSetInitProperties /**< Synonym for #fiftyoneDegreesDataSetInitProperties function. */
#define DataSetInitHeaders fiftyoneDegreesDataSetInitHeaders /**< Synonym for #fiftyoneDegreesDataSetInitHeaders function. */
#define DataSetInitFromFile fiftyoneDegreesDataSetInitFromFile /**< Synonym for #fiftyoneDegreesDataSetInitFromFile function. */
#define DataSetInitInMemory fiftyoneDegreesDataSetInitInMemory /**< Synonym for #fiftyoneDegreesDataSetInitInMemory function. */
#define DataSetGet fiftyoneDegreesDataSetGet /**< Synonym for #fiftyoneDegreesDataSetGet function. */
#define DataSetFree fiftyoneDegreesDataSetFree /**< Synonym for #fiftyoneDegreesDataSetFree function. */
#define DataSetReloadManagerFromMemory fiftyoneDegreesDataSetReloadManagerFromMemory /**< Synonym for #fiftyoneDegreesDataSetReloadManagerFromMemory function. */
#define DataSetReloadManagerFromFile fiftyoneDegreesDataSetReloadManagerFromFile /**< Synonym for #fiftyoneDegreesDataSetReloadManagerFromFile function. */
#define HeadersIsHttp fiftyoneDegreesHeadersIsHttp /**< Synonym for #fiftyoneDegreesHeadersIsHttp function. */
#define ListReset fiftyoneDegreesListReset /**< Synonym for #fiftyoneDegreesListReset function. */
#define ListRelease fiftyoneDegreesListRelease /**< Synonym for #fiftyoneDegreesListRelease function. */
#define ValueGetContent fiftyoneDegreesValueGetContent /**< Synonym for #fiftyoneDegreesValueGetContent function. */
#define ValueGetName fiftyoneDegreesValueGetName /**< Synonym for #fiftyoneDegreesValueGetName function. */
#define ValueGetByName fiftyoneDegreesValueGetByName /**< Synonym for #fiftyoneDegreesValueGetByName function. */
#define ValueGetByNameAndType fiftyoneDegreesValueGetByNameAndType /**< Synonym for #fiftyoneDegreesValueGetByNameAndType function. */
#define ValueGetIndexByName fiftyoneDegreesValueGetIndexByName /**< Synonym for #fiftyoneDegreesValueGetIndexByName function. */
#define ValueGetIndexByNameAndType fiftyoneDegreesValueGetIndexByNameAndType /**< Synonym for #fiftyoneDegreesValueGetIndexByNameAndType function. */
#define ValueGet fiftyoneDegreesValueGet /**< Synonym for #fiftyoneDegreesValueGet function. */
#define CollectionBinarySearch fiftyoneDegreesCollectionBinarySearch /**< Synonym for #fiftyoneDegreesCollectionBinarySearch function. */
#define PropertyGetName fiftyoneDegreesPropertyGetName /**< Synonym for #fiftyoneDegreesPropertyGetName function. */
#define PropertyGetStoredType fiftyoneDegreesPropertyGetStoredType /**< Synonym for #fiftyoneDegreesPropertyGetStoredType function. */
#define PropertyGetStoredTypeByIndex fiftyoneDegreesPropertyGetStoredTypeByIndex /**< Synonym for #fiftyoneDegreesPropertyGetStoredTypeByIndex function. */
#define CollectionReadFileVariable fiftyoneDegreesCollectionReadFileVariable /**< Synonym for #fiftyoneDegreesCollectionReadFileVariable function. */
#define PropertyGetByName fiftyoneDegreesPropertyGetByName /**< Synonym for #fiftyoneDegreesPropertyGetByName function. */
#define ComponentGetKeyValuePair fiftyoneDegreesComponentGetKeyValuePair /**< Synonym for #fiftyoneDegreesComponentGetKeyValuePair function., */
#define PropertyGetValueType fiftyoneDegreesPropertyGetValueType /**< Synonym for #fiftyoneDegreesPropertyGetValueType function. */
#define EvidencePropertiesGetMethod fiftyoneDegreesEvidencePropertiesGetMethod /**< Synonym for #fiftyoneDegreesEvidencePropertiesGetMethod function. */
#define IpiGetCoordinate fiftyoneDegreesIpiGetCoordinate /**< Synonym for #fiftyoneDegreesIpiGetCoordinate function. */
#define SignalCreate fiftyoneDegreesSignalCreate /**< Synonym for #fiftyoneDegreesSignalCreate function. */
#define SignalClose fiftyoneDegreesSignalClose /**< Synonym for #fiftyoneDegreesSignalClose function. */
#define SignalSet fiftyoneDegreesSignalSet /**< Synonym for #fiftyoneDegreesSignalSet function. */
#define SignalWait fiftyoneDegreesSignalWait /**< Synonym for #fiftyoneDegreesSignalWait function. */
#define IntToBool fiftyoneDegreesIntToBool /**< Synonym for #fiftyoneDegreesIntToBool function. */
#define BoolToInt fiftyoneDegreesBoolToInt /**< Synonym for #fiftyoneDegreesBoolToInt function. */
#define ProcessGetId fiftyoneDegreesProcessGetId /**< Synonym for fiftyoneDegreesProcessGetId */
#define YamlFileIterate fiftyoneDegreesYamlFileIterate /**< Synonym for fiftyoneDegreesYamlFileIterate */
#define YamlFileIterateWithLimit fiftyoneDegreesYamlFileIterateWithLimit /**< Synonym for fiftyoneDegreesYamlFileIterateWithLimit */
#define IndicesPropertyProfileCreate fiftyoneDegreesIndicesPropertyProfileCreate /**< Synonym for fiftyoneDegreesIndicesPropertyProfileCreate */
#define IndicesPropertyProfileFree fiftyoneDegreesIndicesPropertyProfileFree /**< Synonym for fiftyoneDegreesIndicesPropertyProfileFree */
#define IndicesPropertyProfileLookup fiftyoneDegreesIndicesPropertyProfileLookup /**< Synonym for fiftyoneDegreesIndicesPropertyProfileLookup */
#define JsonDocumentStart fiftyoneDegreesJsonDocumentStart /**< Synonym for fiftyoneDegreesJsonDocumentStart */
#define JsonDocumentEnd fiftyoneDegreesJsonDocumentEnd /**< Synonym for fiftyoneDegreesJsonDocumentEnd */
#define JsonPropertyStart fiftyoneDegreesJsonPropertyStart /**< Synonym for fiftyoneDegreesJsonPropertyStart */
#define JsonPropertyEnd fiftyoneDegreesJsonPropertyEnd /**< Synonym for fiftyoneDegreesJsonPropertyEnd */
#define JsonPropertyValues fiftyoneDegreesJsonPropertyValues /**< Synonym for fiftyoneDegreesJsonPropertyValues */
#define JsonPropertySeparator fiftyoneDegreesJsonPropertySeparator /**< Synonym for fiftyoneDegreesJsonPropertySeparator */
#define StringBuilderInit fiftyoneDegreesStringBuilderInit /**< Synonym for fiftyoneDegreesStringBuilderInit */
#define StringBuilderAddChar fiftyoneDegreesStringBuilderAddChar /**< Synonym for fiftyoneDegreesStringBuilderAddChar */
#define StringBuilderAddInteger fiftyoneDegreesStringBuilderAddInteger /**< Synonym for fiftyoneDegreesStringBuilderAddInteger */
#define StringBuilderAddDouble fiftyoneDegreesStringBuilderAddDouble /**< Synonym for fiftyoneDegreesStringBuilderAddDouble */
#define StringBuilderAddChars fiftyoneDegreesStringBuilderAddChars /**< Synonym for fiftyoneDegreesStringBuilderAddChars */
#define StringBuilderAddIpAddress fiftyoneDegreesStringBuilderAddIpAddress /**< Synonym for fiftyoneDegreesStringBuilderAddIpAddress */
#define StringBuilderAddStringValue fiftyoneDegreesStringBuilderAddStringValue /**< Synonym for fiftyoneDegreesStringBuilderAddStringValue */
#define StringBuilderComplete fiftyoneDegreesStringBuilderComplete /**< Synonym for fiftyoneDegreesStringBuilderComplete */
#define EvidenceIterateMethod fiftyoneDegreesEvidenceIterateMethod /**< Synonym for fiftyoneDegreesEvidenceIterateMethod */
#define OverrideHasValueForRequiredPropertyIndex fiftyoneDegreesOverrideHasValueForRequiredPropertyIndex /**< Synonym for fiftyoneDegreesOverrideHasValueForRequiredPropertyIndex */
#define IpAddressParse fiftyoneDegreesIpAddressParse /**< Synonym for fiftyoneDegreesIpAddressParse */
#define IpAddressesCompare fiftyoneDegreesIpAddressesCompare /**< Synonym for fiftyoneDegreesIpAddressesCompare */
#define ConvertWkbToWkt fiftyoneDegreesConvertWkbToWkt /**< Synonym for fiftyoneDegreesConvertWkbToWkt */
#define WriteWkbAsWktToStringBuilder fiftyoneDegreesWriteWkbAsWktToStringBuilder /**< Synonym for fiftyoneDegreesWriteWkbAsWktToStringBuilder */
#define WeightedItemListInit fiftyoneDegreesWeightedItemListInit /**< Synonym for fiftyoneDegreesWeightedItemListInit */
#define WeightedItemListRelease fiftyoneDegreesWeightedItemListRelease /**< Synonym for fiftyoneDegreesWeightedItemListRelease */
#define WeightedItemListFree fiftyoneDegreesWeightedItemListFree /**< Synonym for fiftyoneDegreesWeightedItemListFree */
#define WeightedItemListExtend fiftyoneDegreesWeightedItemListExtend /**< Synonym for fiftyoneDegreesWeightedItemListExtend */
#define WeightedItemListAdd fiftyoneDegreesWeightedItemListAdd /**< Synonym for fiftyoneDegreesWeightedItemListAdd */
#define ValueGetWeight fiftyoneDegreesValueGetWeight /**< Synonym for fiftyoneDegreesValueGetWeight */
#define ValueIsWeighted fiftyoneDegreesValueIsWeighted /**< Synonym for fiftyoneDegreesValueIsWeighted */
#define PropertyValueTypeGetUnderlyingType fiftyoneDegreesPropertyValueTypeGetUnderlyingType /**< Synonym for fiftyoneDegreesPropertyValueTypeGetUnderlyingType */

/* <-- only one asterisk to avoid inclusion in documentation
 * Shortened constants.
 */

#define IpAddressStringMaxLength fiftyoneDegreesIpAddressStringMaxLength /**< Synonym for #fiftyoneDegreesIpAddressStringMaxLength constant. */
#define REASONABLE_WKT_STRING_LENGTH FIFTYONE_DEGREES_REASONABLE_WKT_STRING_LENGTH /**< Synonym for #FIFTYONE_DEGREES_REASONABLE_WKT_STRING_LENGTH constant macro. */
#define MAX_DOUBLE_DECIMAL_PLACES FIFTYONE_DEGREES_MAX_DOUBLE_DECIMAL_PLACES /**< Synonym for #FIFTYONE_DEGREES_MAX_DOUBLE_DECIMAL_PLACES constant macro. */

/* <-- only one asterisk to avoid inclusion in documentation
 * Shortened macros.
 */

#define EXCEPTION_SET FIFTYONE_DEGREES_EXCEPTION_SET /**< Synonym for #FIFTYONE_DEGREES_EXCEPTION_SET macro. */
#define EXCEPTION_CLEAR FIFTYONE_DEGREES_EXCEPTION_CLEAR /**< Synonym for #FIFTYONE_DEGREES_EXCEPTION_CLEAR macro. */
#define EXCEPTION_CREATE FIFTYONE_DEGREES_EXCEPTION_CREATE /**< Synonym for #FIFTYONE_DEGREES_EXCEPTION_CREATE macro. */
#define EXCEPTION_OKAY FIFTYONE_DEGREES_EXCEPTION_OKAY /**< Synonym for #FIFTYONE_DEGREES_EXCEPTION_OKAY macro. */
#define EXCEPTION_FAILED FIFTYONE_DEGREES_EXCEPTION_FAILED /**< Synonym for #FIFTYONE_DEGREES_EXCEPTION_FAILED macro. */
#define EXCEPTION_THROW FIFTYONE_DEGREES_EXCEPTION_THROW /**< Synonym for #FIFTYONE_DEGREES_EXCEPTION_THROW macro. */
#define EXCEPTION_CHECK FIFTYONE_DEGREES_EXCEPTION_CHECK /**< Synonym for #FIFTYONE_DEGREES_EXCEPTION_CHECK macro. */
#define STRING FIFTYONE_DEGREES_STRING /**< Synonym for #FIFTYONE_DEGREES_STRING macro. */
#define COLLECTION_RELEASE FIFTYONE_DEGREES_COLLECTION_RELEASE /**< Synonym for #FIFTYONE_DEGREES_COLLECTION_RELEASE macro. */
#define FILE_MAX_PATH FIFTYONE_DEGREES_FILE_MAX_PATH /**< Synonym for #FIFTYONE_DEGREES_FILE_MAX_PATH macro. */
#define THREAD_CREATE FIFTYONE_DEGREES_THREAD_CREATE /**< Synonym for #FIFTYONE_DEGREES_THREAD_CREATE macro. */
#define THREAD_CLOSE FIFTYONE_DEGREES_THREAD_CLOSE /**< Synonym for #FIFTYONE_DEGREES_THREAD_CLOSE macro. */
#define THREAD_EXIT FIFTYONE_DEGREES_THREAD_EXIT /**< Synonym for #FIFTYONE_DEGREES_THREAD_EXIT macro. */
#define THREAD_JOIN FIFTYONE_DEGREES_THREAD_JOIN /**< Synonym for #FIFTYONE_DEGREES_THREAD_JOIN macro. */
#define THREAD_ROUTINE FIFTYONE_DEGREES_THREAD_ROUTINE /**< Synonym for #FIFTYONE_DEGREES_THREAD_ROUTINE macro. */
#define MUTEX_CREATE FIFTYONE_DEGREES_MUTEX_CREATE /**< Synonym for #FIFTYONE_DEGREES_MUTEX_CREATE macro. */
#define MUTEX_CLOSE FIFTYONE_DEGREES_MUTEX_CLOSE /**< Synonym for #FIFTYONE_DEGREES_MUTEX_CLOSE macro. */
#define MUTEX_LOCK FIFTYONE_DEGREES_MUTEX_LOCK /**< Synonym for #FIFTYONE_DEGREES_MUTEX_LOCK macro. */
#define MUTEX_UNLOCK FIFTYONE_DEGREES_MUTEX_UNLOCK /**< Synonym for #FIFTYONE_DEGREES_MUTEX_UNLOCK macro. */
#define MUTEX_VALID FIFTYONE_DEGREES_MUTEX_VALID /**< Synonym for #FIFTYONE_DEGREES_MUTEX_VALID macro. */
#define THREAD FIFTYONE_DEGREES_THREAD /**< Synonym for #FIFTYONE_DEGREES_THREAD macro. */
#define INTERLOCK_INC FIFTYONE_DEGREES_INTERLOCK_INC /**< Synonym for #FIFTYONE_DEGREES_INTERLOCK_INC macro. */
#define INTERLOCK_DEC FIFTYONE_DEGREES_INTERLOCK_DEC /**< Synonym for #FIFTYONE_DEGREES_INTERLOCK_DEC macro. */
#define INTERLOCK_EXCHANGE FIFTYONE_DEGREES_INTERLOCK_EXCHANGE /**< Synonym for #FIFTYONE_DEGREES_INTERLOCK_EXCHANGE macro. */
#define INTERLOCK_EXCHANGE_64 FIFTYONE_DEGREES_INTERLOCK_EXCHANGE_64 /**< Synonym for #FIFTYONE_DEGREES_INTERLOCK_EXCHANGE_64 macro. */
#define INTERLOCK_EXCHANGE_PTR FIFTYONE_DEGREES_INTERLOCK_EXCHANGE_PTR /**< Synonym for #FIFTYONE_DEGREES_INTERLOCK_EXCHANGE_PTR macro. */
#define INTERLOCK_EXCHANGE_PTR_DW FIFTYONE_DEGREES_INTERLOCK_EXCHANGE_PTR_DW /**< Synonym for #FIFTYONE_DEGREES_INTERLOCK_EXCHANGE_PTR_DW macro. */
#define FLOAT_TO_NATIVE FIFTYONE_DEGREES_FLOAT_TO_NATIVE /**< Synonym for #FIFTYONE_DEGREES_FLOAT_TO_NATIVE macro. */
#define NATIVE_TO_FLOAT FIFTYONE_DEGREES_NATIVE_TO_FLOAT /**< Synonym for #FIFTYONE_DEGREES_NATIVE_TO_FLOAT macro. */
#define FLOAT_IS_EQUAL FIFTYONE_DEGREES_FLOAT_IS_EQUAL /**< Synonym for #FIFTYONE_DEGREES_FLOAT_IS_EQUAL macro. */
#define PSEUDO_HEADER_SEP FIFTYONE_DEGREES_PSEUDO_HEADER_SEP /**< Synonym for #FIFTYONE_DEGREES_PSEUDO_HEADER_SEP macro. */
#define REASONABLE_WKT_STRING_LENGTH FIFTYONE_DEGREES_REASONABLE_WKT_STRING_LENGTH /**< Synonym for #FIFTYONE_DEGREES_REASONABLE_WKT_STRING_LENGTH macro. */

/* <-- only one asterisk to avoid inclusion in documentation
 * Shorten status codes.
 */

#define SUCCESS FIFTYONE_DEGREES_STATUS_SUCCESS /**< Synonym for #FIFTYONE_DEGREES_STATUS_SUCCESS status code. */
#define INSUFFICIENT_MEMORY FIFTYONE_DEGREES_STATUS_INSUFFICIENT_MEMORY /**< Synonym for #FIFTYONE_DEGREES_STATUS_INSUFFICIENT_MEMORY status code. */
#define CORRUPT_DATA FIFTYONE_DEGREES_STATUS_CORRUPT_DATA /**< Synonym for #FIFTYONE_DEGREES_STATUS_CORRUPT_DATA status code. */
#define INCORRECT_VERSION FIFTYONE_DEGREES_STATUS_INCORRECT_VERSION /**< Synonym for #FIFTYONE_DEGREES_STATUS_INCORRECT_VERSION status code. */
#define FILE_NOT_FOUND FIFTYONE_DEGREES_STATUS_FILE_NOT_FOUND /**< Synonym for #FIFTYONE_DEGREES_STATUS_FILE_NOT_FOUND status code. */
#define FILE_BUSY FIFTYONE_DEGREES_STATUS_FILE_BUSY /**< Synonym for #FIFTYONE_DEGREES_STATUS_FILE_BUSY status code. */
#define FILE_FAILURE FIFTYONE_DEGREES_STATUS_FILE_FAILURE /**< Synonym for #FIFTYONE_DEGREES_STATUS_FILE_FAILURE status code. */
#define NOT_SET FIFTYONE_DEGREES_STATUS_NOT_SET /**< Synonym for #FIFTYONE_DEGREES_STATUS_NOT_SET status code. */
#define POINTER_OUT_OF_BOUNDS FIFTYONE_DEGREES_STATUS_POINTER_OUT_OF_BOUNDS /**< Synonym for #FIFTYONE_DEGREES_STATUS_POINTER_OUT_OF_BOUNDS status code. */
#define NULL_POINTER FIFTYONE_DEGREES_STATUS_NULL_POINTER /**< Synonym for #FIFTYONE_DEGREES_STATUS_NULL_POINTER status code. */
#define TOO_MANY_OPEN_FILES FIFTYONE_DEGREES_STATUS_TOO_MANY_OPEN_FILES /**< Synonym for #FIFTYONE_DEGREES_STATUS_TOO_MANY_OPEN_FILES status code. */
#define REQ_PROP_NOT_PRESENT FIFTYONE_DEGREES_STATUS_REQ_PROP_NOT_PRESENT /**< Synonym for #FIFTYONE_DEGREES_STATUS_REQ_PROP_NOT_PRESENT status code. */
#define PROFILE_EMPTY FIFTYONE_DEGREES_STATUS_PROFILE_EMPTY /**< Synonym for #FIFTYONE_DEGREES_STATUS_PROFILE_EMPTY status code. */
#define COLLECTION_FAILURE FIFTYONE_DEGREES_STATUS_COLLECTION_FAILURE /**< Synonym for #FIFTYONE_DEGREES_STATUS_COLLECTION_FAILURE status code. */
#define FILE_COPY_ERROR FIFTYONE_DEGREES_STATUS_FILE_COPY_ERROR /**< Synonym for #FIFTYONE_DEGREES_STATUS_FILE_COPY_ERROR status code. */
#define FILE_EXISTS_ERROR FIFTYONE_DEGREES_STATUS_FILE_EXISTS_ERROR /**< Synonym for #FIFTYONE_DEGREES_STATUS_FILE_EXISTS_ERROR status code. */
#define FILE_WRITE_ERROR FIFTYONE_DEGREES_STATUS_FILE_WRITE_ERROR /**< Synonym for #FIFTYONE_DEGREES_STATUS_FILE_WRITE_ERROR status code. */
#define FILE_READ_ERROR FIFTYONE_DEGREES_STATUS_FILE_READ_ERROR /**< Synonym for #FIFTYONE_DEGREES_STATUS_FILE_READ_ERROR. */
#define FILE_PERMISSION_DENIED FIFTYONE_DEGREES_STATUS_FILE_PERMISSION_DENIED /**< Synonym for #FIFTYONE_DEGREES_STATUS_FILE_PERMISSION_DENIED status code. */
#define FILE_PATH_TOO_LONG FIFTYONE_DEGREES_STATUS_FILE_PATH_TOO_LONG /**< Synonym for #FIFTYONE_DEGREES_STATUS_FILE_PATH_TOO_LONG status code. */
#define FILE_END_OF_DOCUMENT FIFTYONE_DEGREES_STATUS_FILE_END_OF_DOCUMENT /**< Synonym for #FIFTYONE_DEGREES_STATUS_FILE_END_OF_DOCUMENT status code. */
#define FILE_END_OF_DOCUMENTS FIFTYONE_DEGREES_STATUS_FILE_END_OF_DOCUMENTS /**< Synonym for #FIFTYONE_DEGREES_STATUS_FILE_END_OF_DOCUMENTS status code. */
#define FILE_END_OF_FILE FIFTYONE_DEGREES_STATUS_FILE_END_OF_FILE /**< Synonym for #FIFTYONE_DEGREES_STATUS_FILE_END_OF_FILE status code. */
#define ENCODING_ERROR FIFTYONE_DEGREES_STATUS_ENCODING_ERROR /**< Synonym for #FIFTYONE_DEGREES_STATUS_ENCODING_ERROR status code. */
#define INVALID_COLLECTION_CONFIG FIFTYONE_DEGREES_STATUS_INVALID_COLLECTION_CONFIG /**< Synonym for #FIFTYONE_DEGREES_STATUS_INVALID_COLLECTION_CONFIG status code. */
#define INVALID_CONFIG FIFTYONE_DEGREES_STATUS_INVALID_CONFIG /**< Synonym for #FIFTYONE_DEGREES_STATUS_INVALID_CONFIG status code. */
#define INSUFFICIENT_HANDLES FIFTYONE_DEGREES_STATUS_INSUFFICIENT_HANDLES /**< Synonym for #FIFTYONE_DEGREES_STATUS_INSUFFICIENT_HANDLES status code. */
#define COLLECTION_INDEX_OUT_OF_RANGE FIFTYONE_DEGREES_STATUS_COLLECTION_INDEX_OUT_OF_RANGE /**< Synonym for #FIFTYONE_DEGREES_STATUS_COLLECTION_INDEX_OUT_OF_RANGE status code. */
#define COLLECTION_OFFSET_OUT_OF_RANGE FIFTYONE_DEGREES_STATUS_COLLECTION_OFFSET_OUT_OF_RANGE /**< Synonym for #FIFTYONE_DEGREES_STATUS_COLLECTION_OFFSET_OUT_OF_RANGE status code. */
#define COLLECTION_FILE_SEEK_FAIL FIFTYONE_DEGREES_STATUS_COLLECTION_FILE_SEEK_FAIL /**< Synonym for #FIFTYONE_DEGREES_STATUS_COLLECTION_FILE_SEEK_FAIL status code. */
#define COLLECTION_FILE_READ_FAIL FIFTYONE_DEGREES_STATUS_COLLECTION_FILE_READ_FAIL /**< Synonym for #FIFTYONE_DEGREES_STATUS_COLLECTION_FILE_READ_FAIL status code. */
#define INCORRECT_IP_ADDRESS_FORMAT FIFTYONE_DEGREES_STATUS_INCORRECT_IP_ADDRESS_FORMAT /**< Synonym for #FIFTYONE_DEGREES_STATUS_INCORRECT_IP_ADDRESS_FORMAT status code. */
#define TEMP_FILE_ERROR FIFTYONE_DEGREES_STATUS_TEMP_FILE_ERROR /**< Synonym for #FIFTYONE_DEGREES_STATUS_TEMP_FILE_ERROR status code. */
#define DATA_FILE_NEEDS_UPDATED FIFTYONE_DEGREES_STATUS_DATA_FILE_NEEDS_UPDATED /**< Synonym for #FIFTYONE_DEGREES_STATUS_DATA_FILE_NEEDS_UPDATED status code. */
#define INSUFFICIENT_CAPACITY FIFTYONE_DEGREES_STATUS_INSUFFICIENT_CAPACITY /**< Synonym for #FIFTYONE_DEGREES_STATUS_INSUFFICIENT_CAPACITY status code. */
#define INVALID_INPUT FIFTYONE_DEGREES_STATUS_INVALID_INPUT /**< Synonym for #FIFTYONE_DEGREES_STATUS_INVALID_INPUT status code.*/
#define UNSUPPORTED_STORED_VALUE_TYPE FIFTYONE_DEGREES_STATUS_UNSUPPORTED_STORED_VALUE_TYPE /**< Synonym for #FIFTYONE_DEGREES_STATUS_UNSUPPORTED_STORED_VALUE_TYPE status code.*/
#define FILE_TOO_LARGE FIFTYONE_DEGREES_STATUS_FILE_TOO_LARGE /**< Synonym for #FIFTYONE_DEGREES_STATUS_FILE_TOO_LARGE status code.*/
#define UNKNOWN_GEOMETRY FIFTYONE_DEGREES_STATUS_UNKNOWN_GEOMETRY /**< Synonym for #FIFTYONE_DEGREES_STATUS_UNKNOWN_GEOMETRY status code.*/
#define RESERVED_GEOMETRY FIFTYONE_DEGREES_STATUS_RESERVED_GEOMETRY /**< Synonym for #FIFTYONE_DEGREES_STATUS_RESERVED_GEOMETRY status code.*/
#define NOT_IMPLEMENTED FIFTYONE_DEGREES_STATUS_NOT_IMPLEMENTED /**< Synonym for #FIFTYONE_DEGREES_STATUS_NOT_IMPLEMENTED. */
#define IPV6_LENGTH FIFTYONE_DEGREES_IPV6_LENGTH /**< Synonym for #FIFTYONE_DEGREES_IPV6_LENGTH macro.*/
#define IPV4_LENGTH FIFTYONE_DEGREES_IPV4_LENGTH /**< Synonym for #FIFTYONE_DEGREES_IPV4_LENGTH macro.*/
#define IP_TYPE_IPV4 FIFTYONE_DEGREES_IP_TYPE_IPV4 /**< Synonym for #FIFTYONE_DEGREES_IP_TYPE_IPV4 enum value.*/
#define IP_TYPE_IPV6 FIFTYONE_DEGREES_IP_TYPE_IPV6 /**< Synonym for #FIFTYONE_DEGREES_IP_TYPE_IPV6 enum value.*/
#define IP_TYPE_INVALID FIFTYONE_DEGREES_IP_TYPE_INVALID /**< Synonym for #FIFTYONE_DEGREES_IP_TYPE_INVALID enum value.*/

 /**
 * @}
 */

#endif
