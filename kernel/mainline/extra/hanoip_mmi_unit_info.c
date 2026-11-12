// SPDX-License-Identifier: GPL-2.0-only
/*
 * Publish the Motorola "mmi_unit_info" (v3) SMEM item that the stock
 * modem firmware reads. Layout and item ID follow Motorola's SM6150 kernel
 * drivers/mmi_info (SMEM_ID_VENDOR0 = 134, 1024 bytes). Values are module
 * parameters so device identifiers are not built into public images.
 */
#include <linux/module.h>
#include <linux/soc/qcom/smem.h>
#include <linux/string.h>

#define MMI_SMEM_ID		134
#define MMI_SMEM_SIZE		1024
#define MMI_UNIT_INFO_VER	3

struct mmi_unit_info {
	u32 version;
	u32 system_rev;
	u32 system_serial_low;
	u32 system_serial_high;
	char machine[33];
	char barcode[65];
	char carrier[65];
	char baseband[97];
	char device[33];
	u32 radio;
	u32 powerup_reason;
	char radio_str[33];
};

static char *device = "hanoip";
static char *barcode = "";
static char *carrier = "";
static char *baseband = "";
static char *radio_str = "";
static uint radio;
static uint system_rev;
static uint serial_low;
static uint serial_high;
static uint powerup_reason;
module_param(device, charp, 0444);
module_param(barcode, charp, 0444);
module_param(carrier, charp, 0444);
module_param(baseband, charp, 0444);
module_param(radio_str, charp, 0444);
module_param(radio, uint, 0444);
module_param(system_rev, uint, 0444);
module_param(serial_low, uint, 0444);
module_param(serial_high, uint, 0444);
module_param(powerup_reason, uint, 0444);

static int __init mmi_unit_info_init(void)
{
	struct mmi_unit_info *mui;
	size_t size;
	int ret;

	BUILD_BUG_ON(sizeof(*mui) > MMI_SMEM_SIZE);

	ret = qcom_smem_alloc(QCOM_SMEM_HOST_ANY, MMI_SMEM_ID, MMI_SMEM_SIZE);
	if (ret && ret != -EEXIST)
		return ret;

	mui = qcom_smem_get(QCOM_SMEM_HOST_ANY, MMI_SMEM_ID, &size);
	if (IS_ERR(mui))
		return PTR_ERR(mui);
	if (size < sizeof(*mui))
		return -EINVAL;

	memset(mui, 0, size);
	mui->version = MMI_UNIT_INFO_VER;
	mui->system_rev = system_rev;
	mui->system_serial_low = serial_low;
	mui->system_serial_high = serial_high;
	strscpy(mui->barcode, barcode, sizeof(mui->barcode));
	strscpy(mui->carrier, carrier, sizeof(mui->carrier));
	strscpy(mui->baseband, baseband, sizeof(mui->baseband));
	strscpy(mui->device, device, sizeof(mui->device));
	mui->radio = radio;
	mui->powerup_reason = powerup_reason;
	strscpy(mui->radio_str, radio_str, sizeof(mui->radio_str));

	pr_info("mmi_unit_info (SMEM %d) for modem: version = 0x%02x, device = '%s', radio_str = '%s', system_rev = 0x%04x, carrier = '%s'\n",
		MMI_SMEM_ID, mui->version, mui->device, mui->radio_str,
		mui->system_rev, mui->carrier);
	return 0;
}
module_init(mmi_unit_info_init);

MODULE_DESCRIPTION("Motorola mmi_unit_info SMEM item for the stock modem");
MODULE_LICENSE("GPL");
